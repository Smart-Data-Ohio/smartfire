#!/usr/bin/env bash
# The Rails app's frontend inputs (app/assets, config/importmap.rb, ...) and test/fixtures are
# relative symlinks into rust/web and rust/fixtures. A container that bind-mounts the app's app/,
# config/ or test/ at /rails needs those targets at /rails/rust/... for the links to resolve.
# Usage: mapfile -t LINK_MOUNTS < <(rails_link_mounts "$REFERENCE_ROOT"); docker run "${LINK_MOUNTS[@]}" ...
# A reference Rails app without rust/ (real directories) gets no extra mounts.
rails_link_mounts() {
  local dir
  for dir in web fixtures; do
    if [ -d "$1/rust/$dir" ]; then printf '%s\n' -v "$(cd "$1/rust/$dir" && pwd):/rails/rust/$dir:ro"; fi
  done
}
