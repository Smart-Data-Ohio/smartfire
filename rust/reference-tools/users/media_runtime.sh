#!/usr/bin/env bash
# Use the oracle's media libraries without changing the host or weakening byte assertions.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
OUT=${WS8BR2_MEDIA_DIR:-$ROOT/../.scratch/rails-media}
IMAGE=${PARITY_IMAGE:-ws8br2-reference:d7c7de92}
mkdir -p "$OUT"
docker run --rm --label parity.owner=ws8br2 --label parity.namespace=ws8br2-media --entrypoint sh "$IMAGE" -c '
  ldd /usr/lib/x86_64-linux-gnu/libvips.so.42 | awk '\''/=> \// { if ($1 !~ /^lib(c|m|pthread|dl|rt|gcc_s|stdc\+\+)\.so/) print $3 }'\'' > /tmp/ws8br2-media-libraries
  printf "/usr/lib/x86_64-linux-gnu/libvips.so.42\n" >> /tmp/ws8br2-media-libraries
  tar -chf - -T /tmp/ws8br2-media-libraries
' > "$OUT/libraries.tar" 2> "$OUT/extract.log"
tar -xf "$OUT/libraries.tar" -C "$OUT"
printf 'WS8br2 pinned media runtime: image %s; libraries extracted; no host libraries changed\n' "$IMAGE"
printf 'LD_LIBRARY_PATH=%s/lib/x86_64-linux-gnu:%s/usr/lib/x86_64-linux-gnu\n' "$OUT" "$OUT"
