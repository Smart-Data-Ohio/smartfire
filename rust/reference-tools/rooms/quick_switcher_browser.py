#!/usr/bin/env python3
"""Original QuickSwitcher declarations, independently of the additional room probes.

room_browser.py still runs its complete drawer/header/thread/pins coverage.
"""
from inbound_browser import run_browser

run_browser('room_browser.mjs', ('--quick-switcher-only',))
