# syntax=mirror.gcr.io/docker/dockerfile:1

# LiveKit v1.13.7 built from its pinned commit with Go 1.27.3, which carries the 2026-10 CA
# root-store fix; the upstream release image is built with Go 1.26.7. Mirrors upstream's
# Dockerfile build. Return to the release image once LiveKit publishes one on a patched Go.
FROM mirror.gcr.io/library/golang:1.27.3-alpine3.24@sha256:f2c872bcee949b7be9c490efd02b48a8a2fbf947cdf533f1c58525ec9d8170fd AS livekit
ARG LIVEKIT_VERSION=v1.13.7
ARG LIVEKIT_COMMIT=8d11efdfcd4220092b6ac7b8a21af28526da5a6b
ARG TARGETARCH
ENV GOTOOLCHAIN=local
WORKDIR /src
RUN apk add --no-cache git && \
    git clone --depth 1 --branch "${LIVEKIT_VERSION}" https://github.com/livekit/livekit.git . && \
    test "$(git rev-parse HEAD)" = "${LIVEKIT_COMMIT}" && \
    go mod download && \
    CGO_ENABLED=0 GOOS=linux GOARCH="${TARGETARCH}" go build -trimpath -o /livekit-server ./cmd/server

FROM mirror.gcr.io/library/node:22-bookworm-slim@sha256:83f487e0a63425e5b4d146fb5e5be574bcbe1b7b843d3ebafdd95eaf7767a7e5

WORKDIR /opt/campfire-huddles/gateway

COPY huddle-gateway/package.json huddle-gateway/package-lock.json ./
RUN npm ci --omit=dev --ignore-scripts && npm cache clean --force

COPY huddle-gateway/gateway.mjs huddle-gateway/server.mjs ./
COPY deploy/huddles/runtime/start.mjs deploy/huddles/runtime/supervisor.mjs deploy/huddles/runtime/gateway-roots.pem /opt/campfire-huddles/runtime/
COPY --from=livekit /livekit-server /usr/local/bin/livekit-server

USER node

ENTRYPOINT ["node", "/opt/campfire-huddles/runtime/start.mjs"]
