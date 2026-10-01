# WS13 invitation inbox dependencies

The two remaining declarations in `test/system/huddle_invitations_test.rb`
need public actions on Rails' single `ActivityItemsController`, declared by
`config/routes.rb:310`. These are not rooms/huddles controller actions.

| Declaration | Request | Rails action and observable result |
|---|---|---|
| the recipient sees an incoming huddle banner and dismissing it marks the item read | `GET /activity/unread_count.json` | `activity_items#unread_count`: the accessible unread count, with no-store/no-cache headers; the sidebar badge reaches 1 |
| same declaration, Dismiss | `PATCH /activity/:id/read` | `activity_items#read`: missing state defaults to read; accessible item receives `read_at`; JSON payload feeds the real banner/indicator refresh |
| joining from the banner marks the item handled, navigates to the DM room, and rings the huddle panel | `PATCH /activity/:id/handled` | `activity_items#handled`: missing state defaults to handled; accessible item receives `handled_at`; the banner then navigates and dispatches `huddle:join` |

`app/javascript/controllers/activity_indicator_controller.js` performs the count
request. `huddle_invitation_controller.js` performs the read/handled PATCH with
`Accept: application/json` and the real CSRF token. Its payload supplies each
path; neither action needs `/activity/:id/open` for these two declarations.

The current concrete owner is **WS11-UI**, branch
`rust/ws11ui-agent-pages`: `wave4/ws11ui-report.md:10` records its inbox index,
unread-count endpoint and `controllers/activity_items.rs` implementation;
line 24 explicitly retains “Inbox continuation: JSON index; open/read/handled
actions” and HuddleGrant presentation / WS13 overdue resolution. Those changes
are not on WS13's main baseline, so all three requests still return 501 here.
The old ActivityItem module header and WS8 plan's WS12 label are historical
generic inbox assignments, not evidence these endpoints have landed.

Ownership audit also read `wave4/ws11.md`, `ws11ui.md`, `ws8bm.md`,
`ws11-report.md`, `ws8bm-report.md`, `decisions.md`, and the `rust/plans/`
inbox references. WS11's current report leaves pages with WS11-UI. WS8b-m's
brief enumerates message controllers and its report retains activity/work/board
501 seams; it does not own these three actions. No WS12 brief exists in the
current delegation tree. Decisions contains no reassignment of these actions.
The current WS11-UI report is the specific handoff, rather than an inferred
assignment from WS13's domain source header.

No inbox route, controller or domain seam is implemented by this continuation.
WS13b landed in main at b908ebc2 and the overlay is deleted. Eight invitation
declarations now run directly on those merged APIs. The lead ruled the two inbox
declarations nonblocking and deferred to WS11-UI. Enable their retained complete
assertions with WS13_ENABLE_INBOX_CASES=1 when those three public routes land.
