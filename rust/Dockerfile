# syntax = docker/dockerfile:1
#
# Production image for the Rust port. A drop-in for the reference image (the Rails app's Dockerfile):
# same user (uid 1000), working directory, storage layout (/rails/storage/{db,files,backups}), env
# vars, ports and ONCE hooks. The binary does Thruster's job itself (crates/kit/src/front): HTTP on
# 80, and with TLS_DOMAIN, HTTPS on 443 with Let's Encrypt certificates cached in
# /rails/storage/thruster, where the reference's Thruster keeps them.
#
#   docker build -t campfire-rust --build-context reference=.. \
#     --build-arg APP_VERSION=... --build-arg GIT_REVISION=... .
#
# The build context is rust/; the reference Rails app (the repository root, or any checkout of it)
# comes in as the named context `reference`, for the assets and public/ files the binary embeds and
# the assets. Runtime hooks live in this tree and honor the storage overrides.
#
# Media: variants and video posters must be byte-identical to the reference's, so libvips and
# ffmpeg are built from the same Debian trixie source packages the reference image ships
# (libvips 8.16.1-1+deb13u1, ffmpeg 7:7.1.5-0+deb13u1), with the same compiler flags, against the
# same Debian libraries. What's left out is only what Campfire never reaches:
#
#   * libvips: the loaders that Vips.block_untrusted and the variable content types already rule
#     out (ImageMagick, OpenSlide, PDF, SVG, JPEG XL, JPEG 2000, OpenEXR, FITS, Matlab), and
#     text rendering, Deep Zoom and FFT. Loads PNG, GIF, JPEG, TIFF, WebP, AVIF and HEIC/HEIF,
#     with EXIF orientation and ICC colour management; saves PNG, JPEG, GIF and WebP.
#   * ffmpeg: every built-in demuxer, decoder and parser, plus dav1d for AV1, so ffprobe and the
#     preview see the same files as before; one encoder and muxer (a JPEG frame on stdout, what
#     ActiveStorage.video_preview_arguments asks for), the filters that command line and
#     autorotation insert, and the file and pipe protocols. No hardware, network, device or
#     external codec libraries (encoders, speech synthesis, Vulkan/LLVM via libplacebo, ...).
#
# No PDF previewers either: the reference has neither poppler nor mupdf, so PDFs aren't
# previewable there.

ARG RUST_VERSION=1.98.1
ARG DEBIAN_RELEASE=trixie
ARG LIBVIPS_VERSION=8.16.1-1+deb13u1
ARG LIBVIPS_DSC_SHA256=60205e00d061b9d8072938e04899f2ca2fdac0513068e561d33f7c87fae1ae2e
ARG FFMPEG_VERSION=7:7.1.5-0+deb13u1
ARG FFMPEG_DSC_SHA256=9ed2ed34cbe7f056eeebbe9045c5e2d15e41b5b053fe7c8ba6979a0b6fb081ce


# Toolchain and Debian -dev packages for libvips, ffmpeg and the Rust build, with deb-src enabled so
# `apt-get source` can fetch the exact Debian sources (the .dsc checksums pin them; dpkg-source
# verifies the tarballs against the .dsc).
FROM docker.io/library/rust:${RUST_VERSION}-${DEBIAN_RELEASE}@sha256:a8a5f0a1e5fe7dfe1d352591e4a1c7dd2c08fd70475cae872cf3458ba0df0546 AS media-base
RUN sed -i 's/^Types: deb$/Types: deb deb-src/' /etc/apt/sources.list.d/debian.sources && \
    apt-get update -qq && \
    apt-get install --no-install-recommends -y \
      dpkg-dev meson ninja-build nasm \
      libglib2.0-dev libexpat1-dev zlib1g-dev libjpeg62-turbo-dev libspng-dev libpng-dev \
      libwebp-dev libtiff-dev libheif-dev libexif-dev liblcms2-dev libcgif-dev libimagequant-dev \
      libhwy-dev libdav1d-dev libbz2-dev liblzma-dev
WORKDIR /usr/src


# libvips with only the loaders and savers above, built into the library (no modules), with
# Debian's build flags (debian/rules: meson, buildtype plain, hardening=+all).
FROM media-base AS vips
ARG LIBVIPS_VERSION
ARG LIBVIPS_DSC_SHA256
RUN apt-get source -qq vips=${LIBVIPS_VERSION} && \
    echo "${LIBVIPS_DSC_SHA256}  vips_${LIBVIPS_VERSION}.dsc" | sha256sum -c - && \
    cd vips-${LIBVIPS_VERSION%-*} && \
    eval "$(DEB_BUILD_MAINT_OPTIONS=hardening=+all dpkg-buildflags --export=sh)" && \
    meson setup build --buildtype=plain --wrap-mode=nodownload --prefix=/opt/vips --libdir=lib \
      --auto-features=disabled -Dmodules=disabled -Dintrospection=disabled -Dcplusplus=false \
      -Ddeprecated=false -Dexamples=false \
      -Djpeg=enabled -Dspng=enabled -Dpng=enabled -Dwebp=enabled -Dtiff=enabled -Dheif=enabled \
      -Dexif=enabled -Dlcms=enabled -Dcgif=enabled -Dimagequant=enabled -Dhighway=enabled \
      -Dzlib=enabled && \
    meson compile -C build -j 4 && \
    meson install -C build --no-rebuild --strip


# ffmpeg and ffprobe (shared libavcodec/libavformat/...) from Debian's source with Debian's
# toolchain and version string (debian/rules), so the vectors' version check still applies.
FROM media-base AS ffmpeg
ARG FFMPEG_VERSION
ARG FFMPEG_DSC_SHA256
RUN apt-get source -qq ffmpeg=${FFMPEG_VERSION} && \
    upstream=${FFMPEG_VERSION#*:} && upstream=${upstream%-*} && revision=${FFMPEG_VERSION##*-} && \
    echo "${FFMPEG_DSC_SHA256}  ffmpeg_${FFMPEG_VERSION#*:}.dsc" | sha256sum -c - && \
    cd ffmpeg-${upstream} && \
    ./configure --prefix=/opt/ffmpeg --extra-version="${revision}" --toolchain=hardened \
      --enable-shared --disable-static --disable-doc --disable-ffplay --disable-avdevice \
      --disable-autodetect --disable-network --disable-hwaccels --disable-devices \
      --enable-libdav1d --enable-zlib --enable-bzlib --enable-lzma \
      --disable-encoders --enable-encoder=mjpeg \
      --disable-muxers --enable-muxer=image2 \
      --disable-protocols --enable-protocol=file,pipe \
      --disable-filters \
      --enable-filter=buffer,buffersink,abuffer,abuffersink,format,aformat,null,anull,scale,aresample \
      --enable-filter=select,loop,trim,transpose,hflip,vflip,rotate,crop && \
    make -j4 && \
    make install && \
    strip --strip-unneeded /opt/ffmpeg/lib/*.so.* /opt/ffmpeg/bin/*


# The toolchain CI runs the tests and clippy in, with the source bind-mounted: the same libvips and
# ffmpeg as the image, so the storage vectors' byte comparisons run rather than skip.
FROM media-base AS toolchain
COPY --from=vips /opt/vips /opt/vips
COPY --from=ffmpeg /opt/ffmpeg /opt/ffmpeg
ENV LIBRARY_PATH=/opt/vips/lib \
    LD_LIBRARY_PATH=/opt/vips/lib:/opt/ffmpeg/lib \
    PATH=/opt/ffmpeg/bin:$PATH
ARG TARGETARCH
# Pin nextest's prebuilt binaries, including their checksums, so the image never compiles its
# runner from source. Keep mold in this CI-only stage; developer and production builds use
# their normal linker unless CI explicitly supplies RUSTFLAGS.
RUN rustup component add clippy && \
    apt-get update -qq && apt-get install --no-install-recommends -y mold && \
    case "$TARGETARCH" in \
      amd64) triple=x86_64-unknown-linux-gnu; checksum=682c21b777c333e96fd532e114d3a5a894e0729ab88d94c0a9f20f8419695428 ;; \
      arm64) triple=aarch64-unknown-linux-gnu; checksum=b2e33d7c72de7ade0ff7b3a948ac37516b24f8a836b7a8870c1f634a94be9de9 ;; \
      *) echo "Unsupported nextest architecture: $TARGETARCH" >&2; exit 1 ;; \
    esac && \
    curl --fail --silent --show-error --location \
      "https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-0.9.146/cargo-nextest-0.9.146-${triple}.tar.gz" \
      --output /usr/src/nextest.tar.gz && \
    echo "${checksum}  /usr/src/nextest.tar.gz" | sha256sum -c - && \
    tar -xzf /usr/src/nextest.tar.gz -C /usr/local/cargo/bin cargo-nextest && \
    rm /usr/src/nextest.tar.gz && \
    rm -rf /var/lib/apt/lists /var/cache/apt/archives


# Build the binary against the libvips above.
FROM media-base AS build
COPY --from=vips /opt/vips /opt/vips
ENV LIBRARY_PATH=/opt/vips/lib
# jemalloc bakes the page size in at build time and refuses to start on a kernel with larger pages.
# arm64 kernels come with 4 or 16 KB pages (the Raspberry Pi 5 uses 16 KB), and built for 16 KB it
# runs on both.
ARG TARGETARCH
ARG CARGO_BUILD_JOBS=4
ARG CARGO_PROFILE=release
ARG CARGO_CACHE_SCOPE=campfire-rust

WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
# crates/assets/build.rs digests and embeds the reference's assets and public/ at build time: only
# those parts of the reference app come in (build.rs leaves out a stray public/assets precompile).
ENV CAMPFIRE_REFERENCE=/reference
COPY --from=reference app/assets /reference/app/assets
COPY --from=reference app/javascript /reference/app/javascript
COPY --from=reference vendor/javascript /reference/vendor/javascript
COPY --from=reference public /reference/public
COPY --from=reference config/importmap.rb /reference/config/importmap.rb
COPY --from=reference config/initializers/assets.rb /reference/config/initializers/assets.rb

RUN --mount=type=cache,id=${CARGO_CACHE_SCOPE}-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=${CARGO_CACHE_SCOPE}-target,target=/src/target \
    if [ "$TARGETARCH" = arm64 ]; then export JEMALLOC_SYS_WITH_LG_PAGE=14; fi && \
    cargo build --profile "$CARGO_PROFILE" --locked -j "$CARGO_BUILD_JOBS" -p campfire && \
    profile_dir="$CARGO_PROFILE" && if [ "$profile_dir" = dev ]; then profile_dir=debug; fi && \
    install -D -m 755 "target/$profile_dir/campfire" /out/campfire


# The shared libraries and executables that go into the runtime, in one directory tree.
FROM scratch AS media
COPY --from=vips /opt/vips/lib/libvips.so.42 /usr/local/lib/
COPY --from=ffmpeg /opt/ffmpeg/lib/libavcodec.so.61 /opt/ffmpeg/lib/libavfilter.so.10 \
     /opt/ffmpeg/lib/libavformat.so.61 /opt/ffmpeg/lib/libavutil.so.59 \
     /opt/ffmpeg/lib/libswresample.so.5 /opt/ffmpeg/lib/libswscale.so.8 /usr/local/lib/
COPY --from=ffmpeg /opt/ffmpeg/bin/ffmpeg /opt/ffmpeg/bin/ffprobe /usr/local/bin/


FROM docker.io/library/debian:${DEBIAN_RELEASE}-slim@sha256:a99cfc517144bc59b1978475ec53b46ecabec7e43635402ee5b77cc54cd1b20a

# ca-certificates: the system CA store, for webhooks, unfurling, Web Push and the ACME directory.
# The rest are the Debian libraries libvips and ffmpeg were built against: glib and expat, the image
# codecs (with libde265 and dav1d as libheif's HEIC and AVIF decoders), lcms2, libexif, cgif and
# libimagequant for GIF saving, and highway for libvips' SIMD paths.
RUN apt-get update -qq && \
    apt-get install --no-install-recommends -y \
      ca-certificates libglib2.0-0t64 libexpat1 libjpeg62-turbo libspng0 libpng16-16t64 \
      libwebp7 libwebpmux3 libwebpdemux2 libtiff6 libheif1 libheif-plugin-libde265 \
      libheif-plugin-dav1d libexif12 liblcms2-2 libcgif0 libimagequant0 libhwy1t64 libdav1d7 && \
    rm -rf /var/lib/apt/lists /var/cache/apt/archives

COPY --from=media / /
RUN ldconfig

# Image metadata
ARG OCI_DESCRIPTION
LABEL org.opencontainers.image.description="${OCI_DESCRIPTION}"
ARG OCI_SOURCE
LABEL org.opencontainers.image.source="${OCI_SOURCE}"
LABEL org.opencontainers.image.licenses="MIT"
LABEL net.smartdata.campfire.runtime="rust"

# Run and own only the runtime files as a non-root user, as the reference does.
RUN groupadd --system --gid 1000 rails && \
    useradd rails --uid 1000 --gid 1000 --create-home --shell /bin/bash

WORKDIR /rails

COPY --from=build /out/campfire /usr/local/bin/campfire

# bin/boot: what the reference's `thrust bin/start-app` did, in one process: HTTP_PORT (80) and,
# with TLS_DOMAIN, HTTPS_PORT (443), with the app itself also on TARGET_PORT (3000, loopback only unless TARGET_BIND says otherwise). Thruster's
# environment (HTTP_*_TIMEOUT, TLS_DOMAIN, ACME_DIRECTORY, CACHE_SIZE, ... and their THRUSTER_
# forms) means the same.
COPY --chmod=755 <<'EOF' /rails/bin/boot
#!/bin/sh
exec /usr/local/bin/campfire server
EOF

# The storage root is Rails.root.join("storage"): storage/db/<env>.sqlite3, storage/files,
# storage/backups.
RUN mkdir -p /rails/storage/db /rails/storage/files /rails/storage/backups && \
    chown -R 1000:1000 /rails

# ONCE and the host release/nightly-backup scripts share the same admin shim.
COPY --chmod=755 ops/prepare-backup /rails/script/admin/prepare-backup
COPY --chmod=755 ops/pre-backup /hooks/pre-backup
COPY --chmod=755 ops/post-restore /hooks/post-restore

USER 1000:1000

# Configure environment defaults
ENV RAILS_ENV="production"
ENV HTTP_IDLE_TIMEOUT=60
ENV HTTP_READ_TIMEOUT=300
ENV HTTP_WRITE_TIMEOUT=300

# Set version and revision
ARG APP_VERSION
ENV APP_VERSION=$APP_VERSION
ARG GIT_REVISION
ENV GIT_REVISION=$GIT_REVISION

# Expose ports for HTTP and HTTPS
EXPOSE 80 443

# Start the server by default, this can be overwritten at runtime
CMD ["bin/boot"]
