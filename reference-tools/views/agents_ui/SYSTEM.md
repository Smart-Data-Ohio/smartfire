`system_behavior.py --binary /absolute/path/to/campfire` runs nine ported Rails
system assertions against Rust, on separate private copies of the frozen `agents_ui`
seed (`parity/bin/frozen-seeds restore`). Build the normal Rust binary; the pinned
Playwright Docker image supplies its committed browser dependencies. Ports come
from host-wide kernel leases (`users/browser_port_leases.py`).

This runner checks behavior; it captures no screenshots and performs no
pixel work. The pinned Rails models once created the original Bender message,
steps and approval, including all parent timestamps and callback-created activity
(`system_fixture.rb` and friends). `record-fixtures.py` recorded that persisted
state as `test-support/agents-ui-fixtures/SCENARIO/patch.sql` plus the labels the
fixture printed (`labels.json`); the runner applies both to the seed copy before
boot. The labels record the Rails DOM identifier too: messages use their client ID
rather than database ID. Failure counts are reported per original Rails file and
cause a nonzero exit.

The directory case includes the agent profile page; the inbox case includes
its persisted approval and ledger decision. These reveal missing shared user
profile/inbox routes rather than treating successful directory/history HTML
renders as complete system parity. The budget scenario boots a separate private copy one day later, so fixture
messages do not count against its initially empty daily window. Rust
receives one real bot-key post and two rejected overflows. Those overflow
requests call the owner budget checker, reproducing the original test’s two
checks without a synthetic database write or callback bypass. Its inbox
contains exactly one notice, and its owner suspends the agent with the kill
switch. `--scenario pages` and `--scenario budget` select the individual groups.
The live draft scenario calls the real Bearer streaming APIs and observes the update
and finalization through the subscribed browser. The work scenario creates, tracks
and assigns work through the human UI, then calls the real Bearer work API and
checks the refreshed status and history. Both use the merged owner producers.

`--scenario work --inject-work-status` and
`--scenario pages --inject-stream-finalize` deliberately corrupt the candidate
SQLite writer while retaining the real UI, endpoint, validations and callbacks.
They must fail the corresponding behavior assertion and return nonzero. These
probes compare no pixels and change no waits or concurrency.

`inbox_lifecycle.sh --check` records twelve raw HTTP responses around real saved-item
reminder dispatch, handling, re-arming the same reminder, and deleting its message.
`check_inbox_lifecycle.py` proves the HTTP replay rejects deliberate faults in
each of those writers, after a passing unmutated baseline. Missing seeds or
failures outside the intended assertions invalidate the check. Deleted-source JSON and HTML 404 bodies and headers now compare against the
installed production exception renderer, including a rejected error-body corruption.

The behavior browser now uses `system_browser.sh`: the same pinned Playwright
image and Unix-socket upstream forwarder as the capture harness, with a stable
`--network none` namespace. The prior host-browser timeout was the work creation
conversation-title assertion (`agent_work_assignment_test.rb:63`, replay line 54
before the wrapper change). Its log contains `ERR_NETWORK_CHANGED` across local
asset and fetch requests: host network changes abort in-flight Chromium requests.
No Rails test or assertion deadline changed. `check_system_network.py` rejects
the former host namespace and holds a POST while an owned Docker bridge appears
and disappears, verifying the isolated browser receives unchanged response bytes.
