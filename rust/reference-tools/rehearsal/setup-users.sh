#!/usr/bin/env bash
#
# Prepares the rehearsal users in the working copy through Rails' own runner (so passwords and the
# Active Record-encrypted TOTP secrets use the rehearsal SECRET_KEY_BASE): A and B get a fresh
# password and a confirmed TOTP credential; C gets a fresh password and no credential, so its first
# sign-in hits enforced enrollment. Credentials go to $REHEARSAL_DIR/work/users.json only.
#
# Usage: REHEARSAL_DIR=... setup-users.sh A_ID B_ID C_ID   (Rails must be running)

set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
: "${REHEARSAL_DIR:?}"
umask 077
"$HERE/rehearsal.sh" runner "
require 'json'
out = {}
{ 'a' => ${1:?A id}, 'b' => ${2:?B id} }.each do |key, id|
  user = User.find(id)
  password = SecureRandom.hex(12)
  user.update!(password: password)
  user.two_factor_credential&.destroy!
  user.two_factor_remembered_devices.delete_all
  secret = TwoFactorCredential.generate_secret
  TwoFactorCredential.create!(user: user, secret: secret, confirmed_at: Time.current)
  out[key] = { id: id, email: user.email_address, password: password, totp_secret: secret }
end
user = User.find(${3:?C id})
password = SecureRandom.hex(12)
user.update!(password: password)
user.two_factor_credential&.destroy!
out['c'] = { id: user.id, email: user.email_address, password: password }
puts 'USERS_JSON=' + JSON.generate(out)
" | sed -n 's/^USERS_JSON=//p' > "$REHEARSAL_DIR/work/users.json"
chmod 644 "$REHEARSAL_DIR/work/users.json"  # readable by the driver container; the data dir is 0700
python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print("users prepared:", ",".join(sorted(d)))' "$REHEARSAL_DIR/work/users.json"
