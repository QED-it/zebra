#!/usr/bin/env python3
"""Read-only zebrad log endpoint. One container, JSON, no actions.

Stdlib only, so a stock python:*-slim image needs no pip install.
"""

import functools
import json
import re
import socket
import http.client
from datetime import datetime, timezone
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlparse

SERVICE_VERSION = "1.0.0"
CONTAINER = "zebra-testnet"
DOCKER_SOCK = "/var/run/docker.sock"
LISTEN_PORT = 8080
DEFAULT_LIMIT = 200
MAX_LIMIT = 500
# How far past container start to look for the startup banner.
STARTUP_WINDOW_SECS = 240

VERSION_PATTERNS = {
    "version": r"version:\s*([^\n]+)",
    "zcash_network": r"Zcash network:\s*([^\n]+)",
    "running_state_version": r"running state version:\s*([^\n]+)",
    "initial_disk_state_version": r"initial disk state version:\s*([^\n]+)",
    "features": r"features:\s*([^\n]+)",
    "target_triple": r"target triple:\s*([^\n]+)",
    "rust_compiler": r"rust compiler:\s*([^\n]+)",
    "rust_release_date": r"rust release date:\s*([^\n]+)",
    "optimization_level": r"optimization level:\s*([^\n]+)",
    "debug_checks": r"debug checks:\s*([^\n]+)",
    "git_tag": r"git tag:\s*([^\n]+)",
    "git_commit_full": r"git commit full:\s*([^\n]+)",
}


class _UnixHTTPConnection(http.client.HTTPConnection):
    """HTTPConnection over a unix socket (the Docker Engine API)."""

    def __init__(self, sock_path, timeout=15):
        super().__init__("localhost", timeout=timeout)
        self._sock_path = sock_path

    def connect(self):
        s = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        s.settimeout(self.timeout)
        s.connect(self._sock_path)
        self.sock = s


def _docker_get(path):
    conn = _UnixHTTPConnection(DOCKER_SOCK)
    try:
        conn.request("GET", path)
        resp = conn.getresponse()
        return resp.status, resp.read()
    finally:
        conn.close()


def _demux(raw, tty):
    """Docker 8-byte-frames stdout/stderr unless the container has a TTY."""
    if tty:
        return raw.decode("utf-8", "replace")
    out = []
    i = 0
    while i + 8 <= len(raw):
        size = int.from_bytes(raw[i + 4 : i + 8], "big")
        i += 8
        out.append(raw[i : i + size].decode("utf-8", "replace"))
        i += size
    return "".join(out)


def _inspect():
    status, body = _docker_get(f"/containers/{CONTAINER}/json")
    if status != 200:
        raise RuntimeError(f"docker inspect {CONTAINER} returned HTTP {status}")
    return json.loads(body)


def _log_lines(query, tty):
    status, body = _docker_get(f"/containers/{CONTAINER}/logs?{query}")
    if status != 200:
        raise RuntimeError(f"docker logs {CONTAINER} returned HTTP {status}")
    text = _demux(body, tty)
    return [ln for ln in text.splitlines() if ln.strip()]


def _extract_version_info(lines):
    return {
        key: next((m.group(1).strip() for ln in lines if (m := re.search(pat, ln))), "Not found")
        for key, pat in VERSION_PATTERNS.items()
    }


@functools.lru_cache(maxsize=1)
def _version_info(started_at, tty):
    try:
        start = datetime.fromisoformat(started_at.replace("Z", "+00:00"))
        since = int(start.timestamp())
    except Exception:
        since = 0
    # Bounded: avoids pulling the whole log history.
    q = f"stdout=1&stderr=1&since={since}&until={since + STARTUP_WINDOW_SECS}"
    return _extract_version_info(_log_lines(q, tty))


def build_payload(limit):
    meta = _inspect()
    tty = bool(meta.get("Config", {}).get("Tty"))
    started_at = meta.get("State", {}).get("StartedAt", "")

    version_info = _version_info(started_at, tty)
    logs = _log_lines(f"stdout=1&stderr=1&tail={limit}", tty)

    return {
        "logStreamName": f"docker/{CONTAINER}",
        "versionInfo": version_info,
        "containerState": meta.get("State", {}).get("Status", "unknown"),
        "containerStartedAt": started_at,
        "logCount": len(logs),
        "logs": logs,
    }


class Handler(BaseHTTPRequestHandler):
    server_version = "zebra-logs-api/" + SERVICE_VERSION

    def _send(self, code, obj):
        body = json.dumps(obj, indent=4).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path.startswith("/healthz"):
            self._send(200, {"ok": True})
            return

        q = parse_qs(urlparse(self.path).query)
        try:
            limit = int(q.get("limit", [DEFAULT_LIMIT])[0])
        except ValueError:
            limit = DEFAULT_LIMIT
        limit = max(1, min(limit, MAX_LIMIT))

        try:
            self._send(200, build_payload(limit))
        except Exception as exc:
            # Detail goes to the container log; the endpoint is public, so the
            # response says nothing about internals.
            self.log_message("error building payload: %s", exc)
            self._send(500, {"error": "internal error"})

    def do_POST(self):
        self._send(405, {"error": "method not allowed"})

    do_PUT = do_DELETE = do_PATCH = do_POST

    def log_message(self, fmt, *args):
        print(f"{datetime.now(timezone.utc).isoformat()} {fmt % args}", flush=True)


if __name__ == "__main__":
    print(
        f"zebra-logs-api {SERVICE_VERSION} container={CONTAINER} port={LISTEN_PORT}",
        flush=True,
    )
    ThreadingHTTPServer(("0.0.0.0", LISTEN_PORT), Handler).serve_forever()
