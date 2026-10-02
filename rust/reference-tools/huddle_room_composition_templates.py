#!/usr/bin/env python3
"""Transcribe static Rails composition markup, binding every request-dependent value.

Input is the recorded Rails corpus; output is native Askama templates, never
runtime fixture HTML. Every asset URL, room id/STI target/name and form token is
replaced by its semantic view helper. All recorded variants are checked in Rust.
"""
import json,re
from pathlib import Path
root=Path(__file__).resolve().parents[2]
vectors=json.loads((root/'rust/crates/campfire/src/controllers/rooms/room_composition_vectors.json').read_text())
out=root/'rust/crates/views/templates/rooms/composition'
out.mkdir(exist_ok=True)
for partial,drive in [('composer','none'),('composer','legacy'),('composer','picker'),('member_panel','none'),('thread_panel','none'),('poll_builder','none')]:
    case=next(c for c in vectors['cases'] if c['partial']==partial and c['input']['drive']==drive)
    html=case['html'];room=case['input']['room']
    html=re.sub(r'<input type="hidden" name="authenticity_token" value="(?:post|patch|delete):([^"\n]+)" />',lambda m:('{{ h::token_tag(self.form_action("'+m[1].split('/')[-1]+'").as_str(), "post") }}' if '/rooms/'+str(room['id'])+'/' in m[1] else '{{ h::token_tag("'+m[1]+'", "post") }}'),html)
    html=html.replace(str(room['id']),'{{ room.id }}')
    html=html.replace('reply_notify_rooms_closed_{{ room.id }}','{{ room.dom_id("reply_notify") }}')
    html=html.replace('Call &lt;&amp;&gt; room','{{ room.display_name }}')
    # The thread panel's shared channel name is supplied separately for unnamed DMs.
    if partial=='thread_panel':html=html.replace('{{ room.display_name }}','{{ self.channel_name() }}')
    html=re.sub(r'/assets/([a-zA-Z0-9_./-]+)-[0-9a-f]{8,}([.][a-zA-Z0-9]+)',lambda m:'{{ ctx.asset("'+m[1]+m[2]+'") }}',html)
    # Askama's normal final newline must not change a captured partial's bytes.
    (out/('_'+partial+('_'+drive if partial=='composer' else '')+'.html')).write_text(html+'{{ "" }}')
