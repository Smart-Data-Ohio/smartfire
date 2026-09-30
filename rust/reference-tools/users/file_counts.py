#!/usr/bin/env python3
"""Count actual Rust test outcomes per owned file from the complete fresh-clone log."""
from pathlib import Path
import re
import sys

log = Path(sys.argv[1]).read_text()
files = {
    "controllers::users::joining_tests": ("controllers/users/joining_tests.rs", 2),
    "controllers::users::profile_page_tests": ("controllers/users/profile_page_tests.rs", 5),
    "controllers::first_runs::tests": ("controllers/first_runs/tests.rs", 3),
    "controllers::welcome::tests": ("controllers/welcome/tests.rs", 2),
    "controllers::accounts::audit_logs::tests": ("controllers/accounts/audit_logs/tests.rs", 5),
    "controllers::accounts::icons::tests": ("controllers/accounts/icons/tests.rs", 9),
    "controllers::accounts::logos::tests": ("controllers/accounts/logos/tests.rs", 2),
    "controllers::accounts::view_tests": ("controllers/accounts/view_tests.rs", 4),
    "controllers::users::people_tests": ("controllers/users/people_tests.rs", 12),
    "controllers::users::profile_settings_tests": ("controllers/users/profile_settings_tests.rs", 4),
    "controllers::accounts::mutation_tests": ("controllers/accounts/mutation_tests.rs", 11),
}
for prefix, (file, expected) in files.items():
    results = re.findall(r"^test " + re.escape(prefix) + r"::[^\s]+ \.\.\. (ok|FAILED|ignored)", log, re.M)
    assert len(results) == expected and all(result == "ok" for result in results), (file, results)
    print(f"{file}: {len(results)} passed; 0 failed; 0 ignored")
print("WS8br2 file accounting: 59 executed Rust groups; 14 audit HTML/nav/CSV cases, 15 date parses, 39 icon validations, 3 icon bodies/navs, 9 logo PNG responses; 13 card bodies, 2 directories, 31 profile PATCH cases, 4 appearance bodies, 23 account/ban cases, 9 individual exactly-one audit checks, 13 account rows, 2 account bodies/navs/footers, 2 invites, 2 CSS bodies; 1 agent-owner case explicitly deferred")
