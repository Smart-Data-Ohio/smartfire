#!/usr/bin/env python3
"""Real picker/group-DM browser acceptance; same isolated servers as inbound acceptance."""
import sys
from inbound_browser import run_browser

run_browser("dm_browser.mjs", tuple(sys.argv[1:]))
