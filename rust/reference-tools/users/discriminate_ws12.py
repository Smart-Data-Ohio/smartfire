#!/usr/bin/env python3
"""Prove the WS12 model assertions reject deliberately broken implementations."""
from pathlib import Path
import os
import subprocess

root = Path(__file__).resolve().parents[2]
scratch = root.parent / ".scratch"
scratch.mkdir(exist_ok=True)
env = os.environ.copy()
env.update(CARGO_BUILD_JOBS="2",TMPDIR=str(scratch))
env.setdefault("CARGO_TARGET_DIR",str(scratch / "target"))
mutations = [
    ("star-self", "crates/db/src/models/user_star.rs", "if user_id == starred_user_id {", "if false {", "campfire_db", "ws12_star_model_enforces_uniqueness_and_self_rejection"),
    ("star-unique", "crates/db/src/models/user_star.rs", "if Self::find(tx.conn(), user_id, starred_user_id)?.is_some() {", "if false {", "campfire_db", "ws12_star_model_enforces_uniqueness_and_self_rejection"),
    ("star-association", "crates/db/src/models/user_star.rs", "if User::find_by_id(tx.conn(), starred_user_id)?.is_none() {", "if false {", "campfire_db", "ws12_star_model_requires_both_associations"),
    ("star-viewer", "crates/db/src/models/user_star.rs", "DELETE FROM user_stars WHERE user_id=? AND starred_user_id=?", "DELETE FROM user_stars WHERE user_id!=? AND starred_user_id=?", "campfire_db", "ws12_star_readers_and_deletes_are_viewer_scoped"),
    ("activity-viewer", "crates/db/src/models/activity_item/access.sql", "AND activity_items.user_id=?1", "AND activity_items.user_id!=?1", "campfire", "ws12_activity_membership_active_human_and_foreign_security_access_match_rails"),
]
for name, relative, old, new, package, test in mutations:
    path = root / relative
    original = path.read_text()
    assert original.count(old)==1, f"mutation anchor moved: {name}"
    try:
        path.write_text(original.replace(old,new))
        result = subprocess.run(["mise","exec","rust@1.98.1","--","cargo","test","--manifest-path",str(root / "Cargo.toml"),
            "-p",package,test,"--","--test-threads=8"],env=env,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
        (scratch / f"ws12-{name}-red.log").write_text(result.stdout)
        lines = [line for line in result.stdout.splitlines() if line.startswith("test result:")]
        assert result.returncode!=0 and any("1 failed" in line for line in lines), f"mutation did not reach a failing assertion: {name}\n{result.stdout[-4000:]}"
        print(f"WS12 mutation {name}: " + lines[-1],flush=True)
    finally:
        path.write_text(original)
print(f"WS12 discrimination: {len(mutations)} mutations rejected; sources restored",flush=True)
