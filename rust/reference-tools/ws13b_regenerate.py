#!/usr/bin/env python3
"""Capture named huddle corpora and the real Rails/Stimulus differential at the current pin."""
import argparse
import json
import os
from pathlib import Path
import subprocess
ROOT = Path(__file__).resolve().parents[2]

OUTPUTS = {'huddle_revocation': 'db/src/tests/huddle_revocation_vectors.json', 'huddle_grant_sequences': 'db/src/tests/huddle_grant_sequence_vectors.json', 'huddle_invitation_sequences': 'db/src/tests/huddle_invitation_sequence_vectors.json', 'huddle_notifier_sequences': 'db/src/tests/huddle_notifier_sequence_vectors.json', 'huddle_domain_lifecycle_sequences': 'db/src/tests/huddle_domain_lifecycle_sequence_vectors.json', 'huddle_membership_creation': 'db/src/tests/huddle_membership_creation_vectors.json', 'huddle_join_push_sequences': 'db/src/tests/huddle_join_push_sequence_vectors.json', 'huddle_job_contracts': 'campfire/src/huddle/huddle_job_contract_vectors.json', 'huddle_render_assertions': 'campfire/src/jobs/huddle_render_assertions.json', 'huddle_query_assertions': 'db/src/tests/huddle_query_assertions.json', 'huddle_neighbor_mention': 'campfire/src/jobs/huddle_neighbor_mention.json', 'huddle_ring_policy_seam': 'db/src/tests/huddle_ring_policy_seam.json', 'ws13b_ring_generations': 'db/src/tests/huddle_ring_generations.json', 'ws13b_review_fixes': 'db/src/tests/ws13b_review_fixes.json', 'huddle_unicode_order': '../vectors/huddle_unicode_order.json', 'huddle_review_fixes': '../vectors/huddle_review_fixes.json', 'huddle_review_recheck': '../vectors/huddle_review_recheck.json'}

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("output", type=Path)
parser.add_argument("--write", action="store_true")
parser.add_argument("--names", default="")
options = parser.parse_args()
options.output = options.output.resolve()
options.output.mkdir(parents=True, exist_ok=True)
pin = (ROOT / "rust/parity/reference.sha").read_text().strip()
for source in ['test/models/huddle_invitation_test.rb', 'test/models/huddle/join_notifier_test.rb', 'test/models/huddle/join_pusher_test.rb', 'test/models/huddle/ring_policy_test.rb']:
    name = source.removeprefix("test/models/").replace("/", "_")
    assert (ROOT / "rust/reference-tools/huddle_pinned" / name).read_bytes() == subprocess.check_output(["git", "show", f"{pin}:{source}"], cwd=ROOT), source
outputs = OUTPUTS
if options.names:
    names = set(options.names.split(","))
    assert names <= outputs.keys(), names - outputs.keys()
    outputs = {name: target for name, target in outputs.items() if name in names}
env = dict(os.environ, PARITY_IMAGE=os.environ.get("PARITY_IMAGE", "campfire-reference"), PARITY_NAMESPACE="pin-refresh-huddle", PARITY_CPUS=os.environ.get("PARITY_CPUS", "1"))
for name, path in outputs.items():
    target = (ROOT / "rust/crates" / path).resolve()
    output = options.output / (name + ".json")
    seed = "default" if name in ("huddle_unicode_order", "huddle_review_fixes", "huddle_review_recheck") else "first_run"
    with output.open("wb") as stdout, (options.output / (name + ".log")).open("wb") as stderr:
        subprocess.run([str(ROOT / "rust/parity/bin/reference"), "runner", "--seed", seed, "--time", "2026-03-02T16:00:00Z", "--freeze", "-e", "RAILS_LOG_LEVEL=fatal", str(ROOT / "rust/reference-tools" / (name + ".rb"))], cwd=ROOT, env=env, stdout=stdout, stderr=stderr, check=True)
    json.loads(output.read_bytes())
    if options.write:
        target.write_bytes(output.read_bytes())
    assert output.read_bytes() == target.read_bytes(), name
    print(f"Rails huddle: {name}; complete captured corpus", flush=True)
if not options.names:
    target = ROOT / "rust/crates/db/src/models/huddle_observed_matrix.json"
    original = json.loads(target.read_text())
    output = options.output / "huddle_observed_matrix.json"
    with (options.output / "observed-matrix.log").open("wb") as log:
        subprocess.run(["python3", str(ROOT / "rust/reference-tools/ws13b_differential.py"), "record", str(original["random_count"]), str(output), ",".join(map(str, original["seeds"]))], cwd=ROOT, env=env, stdout=log, stderr=log, check=True)
    if options.write:
        target.write_bytes(output.read_bytes())
    assert output.read_bytes() == target.read_bytes()
    print(f"Rails/Stimulus observed differential: {len(json.loads(output.read_bytes())['cases'])} independently replayed cases", flush=True)
