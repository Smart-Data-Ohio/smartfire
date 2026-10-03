#!/usr/bin/env python3
"""Require a valid signed cookie to survive the HTTP test browser's expiry parser."""
from pathlib import Path
from discrimination import require_baseline, require_rejected, run_tests

root = Path(__file__).resolve().parents[3]
source = root / "crates/campfire/src/controllers/presenters/accounts/tests.rs"
original = source.read_text()
start = original.index("fn cookie_tombstone(")
end = original.index("\n}\n", start) + 3
broken = '''fn cookie_tombstone(cookie: &str) -> bool {
    cookie.to_ascii_lowercase().contains("max-age=0") || cookie.contains("1970")
}
'''
require_baseline('browser_keeps_valid_signed_cookies_with_epoch_digits_in_the_signature')
try:
    source.write_text(original[:start] + broken + original[end:])
    result = run_tests('browser_keeps_valid_signed_cookies_with_epoch_digits_in_the_signature')
    require_rejected(result, {'browser_keeps_valid_signed_cookies_with_epoch_digits_in_the_signature': ('crates/campfire/src/controllers/presenters/accounts/tests.rs', 'valid signature digits are not an expiry attribute')})
    print('Browser cookie discrimination: valid signed-cookie regression rejected; source restored')
finally:
    source.write_text(original)
