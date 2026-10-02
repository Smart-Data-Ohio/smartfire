`system_behavior.py --binary /absolute/path/to/campfire` runs nine ported Rails
system assertions on separate private copies of the committed `agents_ui`
seed. Build that seed with the pinned image, build the normal Rust binary and
run `npm ci --prefix rust/parity` first. Ports 52797–52799 belong to WS11-ui.

This runner compares behavior; it captures no screenshots and performs no
pixel work. Rails uses the pin plus exactly the approved status-popup layout.
The pinned Rails models create the original Bender message, steps and approval,
including all parent timestamps and callback-created activity. A SQLite backup
copies that exact persisted state to Rust before boot. The fixture records the
Rails DOM identifier too: messages use their client ID rather than database ID. Failure counts are reported per original
Rails file and cause a nonzero exit, even when the other runtime passes.

The directory case includes the agent profile page; the inbox case includes
its persisted approval and ledger decision. These reveal missing shared user
profile/inbox routes rather than treating successful directory/history HTML
renders as complete system parity. The budget scenario boots a separate private copy one day later, so fixture
messages do not count against its initially empty daily window. Both apps
receive one real bot-key post and two rejected overflows. Those overflow
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

`inbox_lifecycle.sh --check` records eleven raw HTTP responses around real saved-item
reminder dispatch, handling, re-arming the same reminder, and deleting its message.
`check_inbox_lifecycle.py` proves the HTTP replay rejects deliberate faults in
each of those writers. The global deleted-source 404 body remains WS9-owned; its
status and the source-removal/access assertions still run.
