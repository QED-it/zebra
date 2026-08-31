# syntax=docker/dockerfile:1
#
# Upstream's server/api/Dockerfile builds with context ./server/api, but the API
# requires siblings outside it — ../../lib/peer-client, ../signals/api — so the
# container exits with MODULE_NOT_FOUND on start. Build from ./server and keep
# the whole tree, with the app at /app/api.
#
# node_modules is installed at /app, not /app/api: those siblings require
# packages from api/package.json (signals/api.js needs express), and Node only
# walks *up* from the requiring file.
ARG NODE_IMAGE=node:22.14.0-bookworm-slim

FROM ${NODE_IMAGE} AS deps
WORKDIR /app
ENV NPM_CONFIG_PRODUCTION=true
COPY api/package.json api/package-lock.json ./
RUN npm ci --omit=dev --no-audit --no-fund

FROM ${NODE_IMAGE} AS runner
ENV NODE_ENV=production \
    PORT=3001
ARG GIT_COMMIT=unknown
LABEL org.opencontainers.image.title="cipherscan-api" \
      org.opencontainers.image.revision="${GIT_COMMIT}"

WORKDIR /app
COPY . /app
COPY --from=deps /app/node_modules /app/node_modules

# The API writes a compact-block cache under /app/cache; it runs as UID 1000
# while everything COPYed is root-owned.
RUN mkdir -p /app/cache && chown -R 1000:1000 /app/cache

# server.js hardcodes server.listen(PORT, '127.0.0.1'), which is right behind a
# same-host nginx but makes the container unreachable from any other container.
RUN sed -i "s/server.listen(PORT, '127.0.0.1'/server.listen(PORT, process.env.BIND_HOST || '0.0.0.0'/" \
      /app/api/server.js \
 && grep -q "process.env.BIND_HOST" /app/api/server.js

WORKDIR /app/api
USER 1000
EXPOSE 3001
CMD ["node", "server.js"]
