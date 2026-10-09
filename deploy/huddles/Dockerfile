# syntax=docker/dockerfile:1

FROM livekit/livekit-server:v1.13.7@sha256:6fd3b7088874c4d119160dd688798dfec852bc014786d392caad15f6f63912a3 AS livekit

FROM node:22-bookworm-slim@sha256:83f487e0a63425e5b4d146fb5e5be574bcbe1b7b843d3ebafdd95eaf7767a7e5

WORKDIR /opt/campfire-huddles/gateway

COPY huddle-gateway/package.json huddle-gateway/package-lock.json ./
RUN npm ci --omit=dev --ignore-scripts && npm cache clean --force

COPY huddle-gateway/gateway.mjs huddle-gateway/server.mjs ./
COPY deploy/huddles/runtime/start.mjs deploy/huddles/runtime/supervisor.mjs /opt/campfire-huddles/runtime/
COPY --from=livekit /livekit-server /usr/local/bin/livekit-server

USER node

ENTRYPOINT ["node", "/opt/campfire-huddles/runtime/start.mjs"]
