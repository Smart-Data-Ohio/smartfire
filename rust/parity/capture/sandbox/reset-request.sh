#!/bin/sh
# reset-request.sh CTRL PORT TARGET URL: asks the host's reset loop (run.sh start_reset_loop) to
# restore the server on PORT, and waits for it. Runs inside the capture image.
set -eu
ctrl=$1 port=$2 target=$3 url=$4
id=$$.$(date +%s%N)
echo "$port $target $url" >"$ctrl/.tmp.$id"
mv "$ctrl/.tmp.$id" "$ctrl/req.$id"
tries=0
while [ ! -e "$ctrl/done.$id" ]; do
  [ -d "$ctrl" ] || { echo "reset host disappeared for $url" >&2; exit 1; }
  heartbeat=$(cat "$ctrl/heartbeat" 2>/dev/null || echo 0)
  now=$(date +%s)
  [ "$((now - heartbeat))" -le 5 ] || { echo "reset host stopped for $url" >&2; exit 1; }
  tries=$((tries + 1))
  [ "$tries" -lt 750 ] || { echo "reset timed out after 75s for $url" >&2; exit 1; }
  sleep 0.1
done
status=$(cat "$ctrl/done.$id"); rm -f "$ctrl/done.$id"
[ "$status" = ok ] || { echo "reset of $url failed" >&2; exit 1; }
