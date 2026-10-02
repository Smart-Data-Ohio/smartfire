`system_behavior.py --binary /absolute/path/to/campfire` runs seven ported Rails
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
Live draft mutation and work assignment remain two explicitly inventoried
cases because their runtime mutation producers are not ported here.
