#!/usr/bin/env python3
"""Inventory pinned owned/shared system files; zero execution, not a parity pass claim."""
from pathlib import Path
import re
import subprocess
ROOT = Path(__file__).resolve().parents[3]
FILES = ['boosting_messages', 'code_highlighting', 'sending_messages', 'threads', 'workspace_markdown',
         'composer', 'composer_attach_menu', 'message_interactions', 'message_actions_mobile',
         'message_toolbar', 'message_list_a11y', 'drive_attachments', 'unread_divider', 'search_forward_edit',
         'keyboard_shortcuts', 'content_security_policy', 'motion', 'mobile_layout', 'timezone_detection']
for name in FILES:
    path = f'test/system/{name}_test.rb'
    source = subprocess.check_output(['git', 'show', f'd7c7de92:{path}'], cwd=ROOT, text=True)
    declared = len(re.findall(r'^\s*test\s+["\']', source, re.M))
    print(f'{path}: {declared} literal test declarations; 0 executed; deferred', flush=True)
print(f'WS8bm system inventory: {len(FILES)} pinned files; declarations only; no browser or pixel pass claim', flush=True)
