# Only the browser/runtime dependencies differ from the pinned Rails oracle.
FROM ws13-reference:sidebar-2e20b24c
USER root
RUN apt-get update -qq && apt-get install -y --no-install-recommends chromium chromium-driver nodejs && rm -rf /var/lib/apt/lists /var/cache/apt/archives
# Docker disallows Chromium's namespace sandbox; retain every test's own flags.
RUN mv /usr/bin/chromium /usr/bin/chromium.package && printf '#!/bin/sh\nexec /usr/bin/chromium.package --no-sandbox "$@"\n' > /usr/bin/chromium && chmod 0755 /usr/bin/chromium
USER 1000:1000
