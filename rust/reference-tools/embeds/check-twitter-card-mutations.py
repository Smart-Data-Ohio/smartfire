#!/usr/bin/env python3
"""Execute card escaping, legacy memo, and lost-job defects against the native tests."""
from pathlib import Path
import os
import re
import subprocess
root = Path(__file__).resolve().parents[2]
env = os.environ.copy()
env.update(CI='1', TMPDIR=str(root.parent/'.scratch'), CARGO_TARGET_DIR=str(root/'.scratch/target'), GITHUB_TEST_PORT_RANGE='51550-51594', CABLE_TEST_PORT_RANGE='51550-51594')
mutants = [
    ('crates/views/src/twitter/cards.rs', r'    fn link\(&self, label:[\s\S]*?(?=    fn view_link)', '    fn link(&self, label: &str, url: &str, class: &str) -> h::Html { h::link_to(url, h::attrs().class(class).attr("target", "_blank").attr("rel", "noopener noreferrer"), label) }\n', 'campfire_views', 'ws15e_x_cards_match_pinned_rails_bytes', 'escaping'),
    ('crates/campfire/src/controllers/presenters/rich_text.rs', r'if let Some\(exists\) = cache.borrow\(\).get\(&post.post_id\) \{\s*return \*exists;\s*\}', '', 'campfire', 'ws15e_x_numeric_order_preloads_a_page_and_memoizes_both_existence_results', 'request memo'),
    ('crates/campfire/src/controllers/presenters.rs', r'if let Some\(posts\) = self.twitter_posts.borrow\(\).get\(&message.id\) \{\s*return Ok\(posts.clone\(\)\);\s*\}', '', 'campfire', 'ws15e_x_numeric_order_preloads_a_page_and_memoizes_both_existence_results', 'preloading'),
]
for index, (file, old, new, package, test, label) in enumerate(mutants, 1):
    path = root/file
    original = path.read_text()
    assert len(re.findall(old, original)) == 1, old
    try:
        mutated = re.sub(old, lambda _: new, original)
        path.write_text(mutated)
        run = subprocess.run(['cargo', 'test', '-j', '4', '-p', package, test, '--', '--nocapture'], cwd=root, env=env, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        (root.parent/'.scratch'/f'twitter-card-mutation-{index}.log').write_text(run.stdout)
        assert run.returncode and 'test result: FAILED' in run.stdout, run.stdout[-2500:]
        print(f'{index} {label}: '+next(line for line in run.stdout.splitlines() if line.startswith('test result:')), flush=True)
    finally:
        path.write_text(original)
print(f'WS15e X card mutation checks: {len(mutants)} detected, 0 survived')
