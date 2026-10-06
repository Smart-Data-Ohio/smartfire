#!/usr/bin/env python3
"""Run alone: compiled merge regressions must fail assertions; restore every edit."""
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[3]
SCRATCH = ROOT / ".scratch" / "discrimination"
SCRATCH.mkdir(parents=True, exist_ok=True)
ENV = dict(os.environ, TMPDIR=str(ROOT / ".scratch"), CARGO_TARGET_DIR=str(ROOT / "rust/target"),
           MAIL_TEST_PORT_RANGE="48000-48049", CABLE_TEST_PORT_RANGE="48000-48049")


def check(name, file, old, new, package, test, suite=None):
    path = ROOT / file
    original = path.read_text()
    assert original.count(old) == 1, f"{name}: ambiguous mutation"
    try:
        path.write_text(original.replace(old, new))
        command = ["cargo", "test", "-j", "4", "-p", package]
        if suite:
            command += ["--test", suite]
        command += [test, "--", "--nocapture"]
        result = subprocess.run(command, cwd=ROOT / "rust", env=ENV, capture_output=True, text=True)
        output = result.stdout + result.stderr
        (SCRATCH / f"{name}.log").write_text(output)
        failures = re.findall(r"^test (\S+) \.\.\. FAILED$", output, re.M)
        summaries = re.findall(r"^test result: FAILED\..*$", output, re.M)
        assert result.returncode and summaries, f"{name}: no failing assertion; see log"
        assert any(t == test or t.endswith("::" + test) for t in failures), f"{name}: wrong test failed"
        print(f"{name}: detected ({len(failures)} failing tests)", flush=True)
        print(summaries[0], flush=True)
    finally:
        path.write_text(original)


check("mail-source", "rust/crates/db/src/models/message.rs",
      "        attributes.markdown_source = Some(source.to_owned());",
      "        let _ = source;", "campfire_db", "mail_markdown_entry_uses_shared_rails_validation_and_source")
check("mail-token-validation", "rust/crates/db/src/models/room.rs",
      "        if self.direct() {\n            crate::models::direct_room::validate_name(self.name.as_deref())?;\n        }\n        loop {",
      "        loop {", "campfire_db", "mail_token_rotation_preserves_direct_name_validation")
check("mail-renderer-boot", "rust/crates/campfire/src/app.rs",
      """    mail.install_renderer(Arc::new({
        let rich_text = rich_text.clone();
        move |conn: &campfire_db::Connection, room: &campfire_db::Room, source: &str| {
            campfire_db::RichText::render_markdown(&*rich_text, conn, source, room.id)
                .map_err(campfire_db::Error::Other)
        }
    }));""", "", "campfire", "ws8_mail_runtime_posts_with_shared_markdown_and_queue")
check("mail-index-atomicity", "rust/crates/db/src/models/message.rs",
      "            message.create_in_index(tx)?;", "            let _ = message.create_in_index(tx);",
      "campfire_mail", "index_failure_rolls_back_mail_post_and_staged_attachment", "inbound")
print("WS8 mail merge discrimination: 4 compiled regressions detected; implementation restored")
