#!/usr/bin/env bash
set -euo pipefail

if [[ $# != 1 ]]; then
  echo "Usage: $0 BASE_URL" >&2
  exit 2
fi
base=${1%/}
if [[ ! $base =~ ^https?://[^/@?#]+$ ]]; then
  echo 'BASE_URL must be an HTTP(S) origin without credentials, a path, or a query' >&2
  exit 2
fi
scratch=$(mktemp -d)
trap 'rm -rf -- "$scratch"' EXIT

fail() {
  echo "MCP OAuth smoke: $*" >&2
  exit 1
}

fetch() {
  local name=$1 expected=$2 path=$3
  shift 3
  local status
  status=$(curl --disable --silent --show-error --max-time 15 \
    --dump-header "$scratch/$name.headers" --output "$scratch/$name.json" \
    --write-out '%{http_code}' "$@" -- "$base$path") || fail "$path request failed"
  [[ $status == "$expected" ]] || fail "$path returned $status, expected $expected"
}

header() {
  awk -v name="$2:" 'tolower($1) == name { sub(/^[^:]+:[[:space:]]*/, ""); sub(/\r$/, ""); print }' "$scratch/$1.headers"
}

allows() {
  header "$1" "$2" | awk -v token="$3" '
    { count = split($0, values, ","); for (i = 1; i <= count; i++) {
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", values[i]);
        if (tolower(values[i]) == tolower(token)) found = 1
    } }
    END { exit !found }
  '
}

fetch server 200 /.well-known/oauth-authorization-server
jq -e --arg base "$base" '
  .issuer == $base and
  .authorization_endpoint == ($base + "/oauth/authorize") and
  .token_endpoint == ($base + "/oauth/token") and
  .registration_endpoint == ($base + "/oauth/register") and
  .revocation_endpoint == ($base + "/oauth/revoke") and
  .response_types_supported == ["code"] and
  (.grant_types_supported | sort) == ["authorization_code", "refresh_token"] and
  .code_challenge_methods_supported == ["S256"] and
  (.token_endpoint_auth_methods_supported | sort) == ["none", "private_key_jwt"] and
  (.revocation_endpoint_auth_methods_supported | sort) == ["none", "private_key_jwt"] and
  (.token_endpoint_auth_signing_alg_values_supported | sort) == ["ES256", "RS256"] and
  (.revocation_endpoint_auth_signing_alg_values_supported | sort) == ["ES256", "RS256"] and
  .client_id_metadata_document_supported == true and
  .authorization_response_iss_parameter_supported == true and
  .scopes_supported == ["agent"]
' "$scratch/server.json" >/dev/null || fail 'authorization-server metadata mismatch'

for path in /.well-known/oauth-protected-resource/agents/mcp /.well-known/oauth-protected-resource; do
  fetch resource 200 "$path"
  jq -e --arg base "$base" '
    .resource == ($base + "/agents/mcp") and
    .authorization_servers == [$base] and
    .scopes_supported == ["agent"] and
    .bearer_methods_supported == ["header"]
  ' "$scratch/resource.json" >/dev/null || fail "$path metadata mismatch"
done

fetch preflight 204 /oauth/register --request OPTIONS \
  --header 'Origin: https://client.example' \
  --header 'Access-Control-Request-Method: POST' \
  --header 'Access-Control-Request-Headers: content-type'
[[ $(header preflight access-control-allow-origin) == '*' ]] || fail 'preflight CORS origin is not *'
[[ -z $(header preflight set-cookie) ]] || fail 'preflight sets cookies'
[[ -z $(header preflight access-control-allow-credentials) ]] || fail 'preflight allows credentials'
allows preflight access-control-allow-methods POST || fail 'preflight does not allow POST'
allows preflight access-control-allow-headers content-type || fail 'preflight does not allow content-type'

fetch mcp 401 /agents/mcp --header 'Content-Type: application/json' \
  --data '{"jsonrpc":"2.0","id":1,"method":"tools/list"}'
challenge=$(header mcp www-authenticate)
[[ $challenge == "Bearer resource_metadata=\"$base/.well-known/oauth-protected-resource/agents/mcp\", scope=\"agent\"" ]] || fail 'MCP challenge mismatch'
echo "MCP OAuth smoke passed for $base"
