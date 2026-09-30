#!/usr/bin/env python3
"""Record exact unmatched owner card bytes for review; never changes or accepts a render."""
import argparse
import json
from pathlib import Path
import re

parser=argparse.ArgumentParser()
parser.add_argument('--capture-log',type=Path,required=True)
args=parser.parse_args()
root=Path(__file__).resolve().parents[3]
line=next(line for line in args.capture_log.read_text().splitlines() if line.startswith('WS8BR_NATIVE_COMPONENTS:'))
actual=json.loads(line.split(':',1)[1])
expected=json.loads((root/'rust/crates/views/tests/golden/rooms/native_components.json').read_text())['rows']
def container(text,selector):
    start=text.index(f'<div id="{selector}"');depth=0
    for tag in re.finditer(r'</?div\b[^>]*>',text[start:]):
        depth+=-1 if tag[0].startswith('</') else 1
        if depth==0:return text[start:start+tag.end()]
    raise ValueError(f'unclosed container {selector}')
regions=[]
for rails,rust in zip(expected,actual,strict=True):
    assert (rails['room_id'],rails['user_id'])==(rust['room_id'],rust['user_id'])
    a,b=rust['message_list'],rails['message_list']
    # Reconstruction is a diagnostic proving the exact extent of this one failure.
    # The strict native_components_check.py still rejects it with exit 1.
    reconstructed=a
    for match in re.finditer(r'<div id="((?:github_pr_cards|fizzy_cards|link_embed_cards|linkedin_cards)_message_[^"]+)"',b):
        selector=match[1];aa=container(a,selector);bb=container(b,selector)
        if aa!=bb:
            assert aa.endswith('></div>'),f'expected an empty missing owner slot: {selector}'
            regions.append(dict(room_id=rust['room_id'],slot=selector,rust_bytes=len(aa.encode()),rails_bytes=len(bb.encode()),rust=aa,rails=bb))
            reconstructed=reconstructed.replace(aa,bb)
    assert reconstructed==b,'unexplained non-card differences remain'
output=dict(reference='d7c7de92',scope='Unmatched card regions only; diagnostic, not a parity fixture or mask',
            designers_list_rust_bytes=len(actual[0]['message_list'].encode()),
            designers_list_rails_bytes=len(expected[0]['message_list'].encode()),regions=regions)
(root/'rust/plans/ws8br-native-residual.json').write_text(json.dumps(output,indent=2)+'\n')
for region in regions:
    print(f"{region['slot']}: Rust {region['rust_bytes']} bytes; Rails {region['rails_bytes']} bytes")
print(f'Native residual inventory: {len(regions)} empty owner card slots explain the complete remaining difference; strict acceptance still ' + ('fails' if regions else 'passes'))
