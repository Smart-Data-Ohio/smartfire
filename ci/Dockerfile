# CI-only prerequisites. Production and ordinary local builds do not need browsers or Docker.
ARG BASE_IMAGE=campfire-toolchain
FROM docker.io/library/node:26.10.0-trixie-slim@sha256:ec7758ee051e457b468b32bde57b0879010b325bb9862718e9615225ce4aaae1 AS node
FROM docker.io/library/docker:29.8.1-cli@sha256:018edbc908e08fcc9dbf029c812c34251e9b4719e6f71ca0e5eae2a987d014ca AS docker
FROM mcr.microsoft.com/playwright:v1.63.0-noble@sha256:eff16c30e6f3f4af0a03fa4b706120d5e9b0891c344a27d64559aff5900a4a27 AS browsers

FROM ${BASE_IMAGE}
COPY --from=node /usr/local/bin/node /usr/local/bin/node
COPY --from=node /usr/local/lib/node_modules /usr/local/lib/node_modules
COPY --from=docker /usr/local/bin/docker /usr/local/bin/docker
COPY --from=docker /usr/local/libexec/docker/cli-plugins/docker-buildx /usr/local/libexec/docker/cli-plugins/docker-buildx
COPY --from=browsers /ms-playwright/chromium-1243 /ms-playwright/chromium-1243
COPY --from=browsers /ms-playwright/chromium_headless_shell-1243 /ms-playwright/chromium_headless_shell-1243
ENV PLAYWRIGHT_BROWSERS_PATH=/ms-playwright PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1
COPY ci/install_apt.py ci/apt-amd64.lock.json /opt/ci/
RUN ln -s ../lib/node_modules/npm/bin/npm-cli.js /usr/local/bin/npm && \
    ln -s ../lib/node_modules/npm/bin/npx-cli.js /usr/local/bin/npx && \
    python3 /opt/ci/install_apt.py /opt/ci/apt-amd64.lock.json && \
    curl --fail --silent --show-error --location \
      https://storage.googleapis.com/chrome-for-testing-public/153.0.8010.12/linux64/chromedriver-linux64.zip \
      --output /tmp/chromedriver.zip && \
    echo 'b7d5f7c120f7827f3538b416e08b82418eb702f18b680c183f0411c8d7f2df69  /tmp/chromedriver.zip' | sha256sum -c - && \
    unzip -q /tmp/chromedriver.zip -d /tmp && \
    install -m 755 /tmp/chromedriver-linux64/chromedriver /usr/local/bin/chromedriver && \
    mkdir -p /usr/lib/chromium && \
    ln -s /ms-playwright/chromium-1243/chrome-linux64/chrome /usr/lib/chromium/chromium && \
    rm -rf /tmp/chromedriver* /var/lib/apt/lists /var/cache/apt/archives

# Native Selenium's namespace supervisor needs --forward-signals (2.42+).
# Build only unshare; use the locked Debian runtime libraries above.
RUN curl --fail --silent --show-error --location \
      https://www.kernel.org/pub/linux/utils/util-linux/v2.42/util-linux-2.42.4.tar.xz \
      --output /tmp/util-linux.tar.xz && \
    echo 'fbd62a100ab7bb8746ba0661255c3c48185b1e9021507c624da01fbc696330ec  /tmp/util-linux.tar.xz' | sha256sum -c - && \
    tar -xf /tmp/util-linux.tar.xz -C /tmp && \
    cd /tmp/util-linux-2.42.4 && \
    ./configure --disable-all-programs --enable-unshare --disable-shared --enable-static --disable-nls && \
    make -j 4 unshare && \
    install -s -m 755 unshare /usr/local/bin/unshare && \
    /usr/local/bin/unshare --help | grep -q -- --forward-signals && \
    cd / && rm -rf /tmp/util-linux*
