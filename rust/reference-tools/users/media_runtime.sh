#!/usr/bin/env bash
# Use the oracle's media libraries and tools without changing the host or byte assertions.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
OUT=${WS8BR2_MEDIA_DIR:-$ROOT/../.scratch/rails-media}
IMAGE=${PARITY_IMAGE:-ws8br2-reference:d7c7de92}
mkdir -p "$OUT"
docker run --rm --label parity.owner=ws8br2 --label parity.namespace=ws8br2-media --entrypoint sh "$IMAGE" -c '
  for input in /usr/lib/x86_64-linux-gnu/libvips.so.42 /usr/bin/ffmpeg /usr/bin/ffprobe; do
    ldd "$input" | awk '\''/=> \// { if ($1 !~ /^lib(c|m|pthread|dl|rt|gcc_s|stdc\+\+)\.so/) print $3 }'\''
    printf "%s\n" "$input"
  done | sort -u > /tmp/ws8br2-media-libraries
  tar -chf - -T /tmp/ws8br2-media-libraries
' > "$OUT/libraries.tar" 2> "$OUT/extract.log"
tar -xf "$OUT/libraries.tar" -C "$OUT"
# Some Debian libraries have absolute RUNPATHs to subdirectories (e.g. PulseAudio).
# Keep every selected soname in one private loader directory rather than resolving a
# host library or relying on the image's absolute paths after extraction.
mkdir -p "$OUT/native-libs"
while IFS= read -r input; do
  case "$input" in
    *.so|*.so.*)
      destination="$OUT/native-libs/$(basename "$input")"
      if [ -e "$destination" ] && ! cmp -s "$OUT/$input" "$destination"; then
        printf 'media runtime: conflicting soname %s\n' "$input" >&2
        exit 1
      fi
      cp "$OUT/$input" "$destination"
      ;;
  esac
done < <(tar -tf "$OUT/libraries.tar")
printf 'WS8br2 pinned media runtime: image %s; libvips, FFmpeg tools and libraries extracted; no host libraries changed\n' "$IMAGE"
printf 'LD_LIBRARY_PATH=%s/native-libs\n' "$OUT"
printf 'PATH=%s/usr/bin:$PATH\n' "$OUT"
