#!/usr/bin/env python3
"""Reject privilege widening, cross-agent revocation and duplicate-grant regressions."""
import os
from pathlib import Path
import subprocess
root=Path(__file__).resolve().parents[3]
sources={name:root/f"crates/campfire/src/controllers/accounts/bots/{name}.rs" for name in ["credentials","grants"]}
originals={name:path.read_text() for name,path in sources.items()}
cases=[
    ("credential-issuance","credentials","concerns::ensure_can_administer(c)?;","","credential_owner_with_sudo_still_cannot_issue"),
    ("grant-widening","grants","concerns::ensure_can_administer(c)?;","","grants_owner_views_and_revokes_but_cannot_widen"),
    ("credential-scope","credentials",".filter(|c| c.agent_id == agent.id)","","credential_and_grant_revocation_are_scoped_to_requested_bot"),
    ("grant-scope","grants",".filter(|g| g.agent_id == agent.id)","","credential_and_grant_revocation_are_scoped_to_requested_bot"),
    ("grant-replay","grants","if existing.is_none() {","if true || existing.is_none() {","grants_create_is_idempotent_and_revocation_disables_legacy_fallback"),
]
try:
    for label,name,needle,replacement,test in cases:
        original=originals[name]
        assert original.count(needle)==1, f"{label}: source changed; review mutation"
        sources[name].write_text(original.replace(needle,replacement))
        result=subprocess.run(["mise","exec","rust@1.98.1","--","cargo","test","--locked","-j","4","-p","campfire",test,"--","--nocapture"],cwd=root,env={**os.environ,"CI":"1"},capture_output=True,text=True)
        output=result.stdout+result.stderr
        assert result.returncode!=0 and "test result: FAILED." in output and "panicked at" in output, f"{label}: mutation escaped test\n{output}"
        print(label+": "+next(line for line in output.splitlines() if line.startswith("test result:")),flush=True)
        sources[name].write_text(original)
    print("Bot access discrimination: 5 regressions detected; sources restored")
finally:
    for name,path in sources.items():path.write_text(originals[name])
