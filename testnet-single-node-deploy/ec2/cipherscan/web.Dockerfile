# syntax=docker/dockerfile:1
#
# Upstream's Dockerfile with two changes, both about pointing the frontend at
# our own API instead of upstream's public one:
#
# 1. NEXT_PUBLIC_API_URL as a build ARG. Next inlines NEXT_PUBLIC_* into the
#    browser bundle at build time, so runtime env alone does nothing. Upstream
#    declares only NEXT_PUBLIC_NETWORK.
#
# 2. lib/api-config.ts patched. Even with the ARG set, upstream hardcodes
#    POSTGRES_API_URLS['testnet'] = 'https://api.testnet.cipherscan.app' and
#    only lets the crosslink network override its URL — so the frontend reads
#    the public Zcash testnet and ignores this node entirely. The patch makes
#    the testnet entry honour NEXT_PUBLIC_API_URL the same way.
ARG NODE_IMAGE=node:22.14.0-bookworm-slim

FROM ${NODE_IMAGE} AS deps
WORKDIR /app
ENV NEXT_TELEMETRY_DISABLED=1
COPY package.json package-lock.json ./
RUN npm ci --no-audit --no-fund

FROM ${NODE_IMAGE} AS builder
WORKDIR /app
ENV NEXT_TELEMETRY_DISABLED=1
ARG GIT_COMMIT=unknown
ARG NEXT_PUBLIC_NETWORK
ARG NEXT_PUBLIC_API_URL
ENV NEXT_PUBLIC_GIT_COMMIT=${GIT_COMMIT} \
    NEXT_PUBLIC_NETWORK=${NEXT_PUBLIC_NETWORK} \
    NEXT_PUBLIC_API_URL=${NEXT_PUBLIC_API_URL}
COPY --from=deps /app/node_modules ./node_modules
COPY . .

# normalizeApiBaseUrl is already imported in this file. grep -q fails the build
# if upstream reworded the line, rather than silently shipping the public URL.
RUN sed -i "s|'testnet': 'https://api.testnet.cipherscan.app',|'testnet': normalizeApiBaseUrl(process.env.NEXT_PUBLIC_API_URL \|\| 'https://api.testnet.cipherscan.app'),|" lib/api-config.ts \
 && grep -q "'testnet': normalizeApiBaseUrl" lib/api-config.ts

RUN npm run build

FROM ${NODE_IMAGE} AS runner
WORKDIR /app
ENV NODE_ENV=production \
    NEXT_TELEMETRY_DISABLED=1 \
    PORT=3000 \
    HOSTNAME=0.0.0.0
ARG GIT_COMMIT=unknown
LABEL org.opencontainers.image.title="cipherscan-web" \
      org.opencontainers.image.revision="${GIT_COMMIT}"
USER 1000
COPY --from=builder --chown=node:node /app/.next/standalone ./
COPY --from=builder --chown=node:node /app/.next/static ./.next/static
COPY --from=builder --chown=node:node /app/public ./public
EXPOSE 3000
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
  CMD node -e "fetch('http://127.0.0.1:'+(process.env.PORT||3000)).then(r=>process.exit(r.ok?0:1)).catch(()=>process.exit(1))"
CMD ["node", "server.js"]
