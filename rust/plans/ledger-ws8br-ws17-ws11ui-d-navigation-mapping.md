# Navigation original assertion mapping

Pinned source: `78b9b1546bdab4c6c1c9b8ddb94512f661289112`. 48 legacy declarations and one current-pin supplementary declaration; 254 legacy direct assertion sites and 129 helper expansions. The supplementary live-unread case has four direct sites and receives no additional legacy closure credit.

All finder assertions call the pinned Selenium displayed atom. Visible text uses the byte-verified ChromeDriver GET_TEXT atom; explicit `visible: :all` applies the exact pinned Capybara Selenium normalization to `textContent`. Hidden assertions require a hidden matching node. Original outer-window sizes are converted through the run-specific native WebDriver measurements; fullscreen retains its real native screen. Geometry reads the exact original DOM-first selector and predicate.

The registered correctness wrapper is `controllers::ledger_browser_tests::original_ledger_navigation_assertions`. Both final paired runs passed at nextest -j 4 (default and CI profiles); final producer controls use the verified pure original fixture seed with original Rails test environment and class-local configuration.

Common authentication visits the original `/test_session` GET with actual email/password credentials and creates a verified server session and expands the original `sign_in` assertion at `test/test_helpers/system_test_helper.rb:65`. Explicit joins preserve all-stream connection and PWA dismissal guards. Message actions preserve `within_message` using `Message#to_key` (client_message_id), then the first Selenium-displayed body match. Confirmation text and button assertions remain inside the original menus/dialog scopes. Persisted predicates read the same server database at the original assertion moment.

| Legacy ID | Original declaration | Browser case | Direct sites | Helper expansions | Setup expansions |
|---|---|---|---:|---:|---:|
| P0308 | `test/system/keyboard_shortcuts_test.rb:9` — ? opens the shortcut sheet everywhere except while typing | `key-help` | 7 | 0 | 1 |
| P0309 | `test/system/keyboard_shortcuts_test.rb:29` — ? while typing stays in the composer | `key-typing-help` | 2 | 0 | 1 |
| P0310 | `test/system/keyboard_shortcuts_test.rb:38` — ctrl+k fires while typing and inserts no link markup | `key-typing-switcher` | 2 | 0 | 1 |
| P0311 | `test/system/keyboard_shortcuts_test.rb:50` — alt+up and alt+down move between rooms | `key-rooms` | 3 | 1 | 1 |
| P0312 | `test/system/keyboard_shortcuts_test.rb:66` — alt+shift+arrows jump between unread rooms | `key-unread-rooms` | 1 | 1 | 1 |
| P0313 | `test/system/keyboard_shortcuts_test.rb:79` — escape marks the current room read | `key-read` | 1 | 6 | 1 |
| P0314 | `test/system/keyboard_shortcuts_test.rb:89` — escape while typing leaves the room unread | `key-typing-escape` | 1 | 6 | 1 |
| P0315 | `test/system/keyboard_shortcuts_test.rb:100` — escape with a menu open closes the menu instead of marking read | `key-menu-escape` | 2 | 8 | 1 |
| P0316 | `test/system/keyboard_shortcuts_test.rb:112` — escape with the huddle theater open yields to the theater instead of marking read | `key-theater` | 3 | 7 | 1 |
| P0317 | `test/system/keyboard_shortcuts_test.rb:149` — escape in full screen leaves the room unread | `key-fullscreen` | 5 | 5 | 1 |
| P0318 | `test/system/keyboard_shortcuts_test.rb:190` — alt+arrows while typing stay in the room | `key-typing-arrows` | 3 | 0 | 1 |
| P0319 | `test/system/keyboard_shortcuts_test.rb:205` — ctrl+k ignores IME composition | `key-ime` | 1 | 0 | 1 |
| P0320 | `test/system/keyboard_shortcuts_test.rb:217` — ctrl+k does not open over an open modal dialog | `key-modal` | 4 | 0 | 1 |
| P0321 | `test/system/keyboard_shortcuts_test.rb:232` — ctrl+k still toggles the switcher closed | `key-toggle` | 2 | 0 | 1 |
| P0384 | `test/system/sidebar_room_menu_test.rb:4` — an admin deletes a channel from the room menu | `menu-admin-delete` | 5 | 1 | 1 |
| P0385 | `test/system/sidebar_room_menu_test.rb:23` — the creator sees Delete on their own room but not on others | `menu-creator` | 3 | 2 | 1 |
| P0386 | `test/system/sidebar_room_menu_test.rb:37` — a plain member does not see Delete | `menu-member` | 1 | 1 | 1 |
| P0387 | `test/system/sidebar_room_menu_test.rb:45` — Cancel and Escape keep the room | `menu-cancel` | 6 | 2 | 1 |
| P0388 | `test/system/sidebar_room_menu_test.rb:67` — deleting the room you are in navigates you home | `menu-current-delete` | 2 | 1 | 1 |
| P0389 | `test/system/sidebar_room_menu_test.rb:83` — Delete is offered on every room kind for admins | `menu-kinds-delete` | 4 | 2 | 1 |
| P0390 | `test/system/sidebar_room_menu_test.rb:108` — a group dm shows Delete only for admins | `menu-group-permission` | 2 | 2 | 2 |
| P0391 | `test/system/sidebar_room_menu_test.rb:125` — one-to-one dms follow the server delete rule | `menu-dm-permission` | 3 | 2 | 1 |
| P0392 | `test/system/sidebar_room_menu_test.rb:140` — Delete is reachable by arrow keys | `menu-keyboard` | 4 | 2 | 1 |
| P0393 | `test/system/sidebar_room_menu_test.rb:159` — deleting from the long-press menu on phones | `menu-phone-delete` | 4 | 0 | 1 |
| P0394 | `test/system/sidebar_room_menu_test.rb:184` — a member leaves an open channel and rejoins through the join page | `menu-open-leave` | 7 | 2 | 1 |
| P0395 | `test/system/sidebar_room_menu_test.rb:209` — leaving a private channel warns about being re-added | `menu-private-leave` | 4 | 2 | 1 |
| P0396 | `test/system/sidebar_room_menu_test.rb:229` — leaving the room you are in sends you home | `menu-current-leave` | 1 | 1 | 1 |
| P0397 | `test/system/sidebar_room_menu_test.rb:244` — the last member out leaves the room behind for no one to lose | `menu-solo-leave` | 2 | 2 | 1 |
| P0398 | `test/system/sidebar_room_menu_test.rb:257` — Leave is offered on every room kind and never deletes | `menu-kinds-leave` | 2 | 2 | 1 |
| P0399 | `test/system/sidebar_room_menu_test.rb:292` — a member leaves a group dm from the menu | `menu-group-leave` | 2 | 2 | 1 |
| P0293 | `test/system/channel_navigation_test.rb:11` — switching channels hides the progress bar and lands without a reload | `nav-switch` | 8 | 5 | 1 |
| P0294 | `test/system/channel_navigation_test.rb:35` — browser back returns to the previous channel without a reload | `nav-back` | 5 | 1 | 1 |
| P0295 | `test/system/channel_navigation_test.rb:54` — slow back and forward restores never flash the channel loader | `nav-history` | 7 | 4 | 1 |
| P0296 | `test/system/channel_navigation_test.rb:79` — canceling a channel visit does not suppress the next page loader | `nav-cancel` | 5 | 2 | 1 |
| P0297 | `test/system/channel_navigation_test.rb:96` — shared message links also navigate without a channel loader | `nav-shared` | 5 | 4 | 1 |
| P0298 | `test/system/channel_navigation_test.rb:111` — channel switch from the mobile drawer closes the drawer without a reload | `nav-phone` | 4 | 1 | 1 |
| P0299 | `test/system/channel_navigation_test.rb:131` — non-room visits still show the progress bar | `nav-nonroom` | 4 | 1 | 1 |
| P0373 | `test/system/room_header_test.rb:18` — phone header keeps members, call, search and More with the rest in the menu | `header-phone` | 28 | 14 | 1 |
| P0374 | `test/system/room_header_test.rb:71` — tablet header keeps notifications and settings visible with the rest in the menu | `header-tablet` | 20 | 9 | 1 |
| P0375 | `test/system/room_header_test.rb:112` — desktop header is unchanged with no More button | `header-desktop` | 12 | 8 | 1 |
| P0376 | `test/system/room_header_test.rb:140` — every phone overflow menu item works | `header-items` | 19 | 0 | 1 |
| P0377 | `test/system/room_header_test.rb:237` — the overflow menu scrolls within a landscape phone viewport | `header-landscape` | 6 | 1 | 1 |
| P0378 | `test/system/room_header_test.rb:279` — opening threads from the menu returns focus to More on close | `header-return-focus` | 3 | 1 | 1 |
| P0379 | `test/system/room_header_test.rb:299` — the overflow menu is keyboard accessible and closes on outside tap | `header-keyboard` | 9 | 3 | 1 |
| P0380 | `test/system/room_header_test.rb:328` — the overflow dot lights for unread threads, not pins, and the pins badge hides at zero | `header-dot` | 8 | 0 | 1 |
| P0381 | `test/system/room_header_test.rb:376` — the tour restarts from the overflow menu on phones | `header-tour` | 9 | 0 | 1 |
| P0411 | `test/system/unread_rooms_test.rb:8` — sending messages between two users | `unread-between` | 3 | 3 | 2 |
| P0412 | `test/system/unread_rooms_test.rb:47` — channel and DM rows keep exactly one unread badge next to the huddle stack | `unread-badges` | 10 | 12 | 2 |

Supplementary current-pin case: `test/system/unread_rooms_test.rb:28`, `unread-live`, with direct sites41–44 and the exact same-room event guard at129. Its full receipts live in `current_pin_records`.

The JSON manifest records exact Ruby text, pinned source SHA256, executable browser source anchors, private/helper/setup expansion sources, synchronized prerequisites, original declaration bodies and outer-window matrices. `d-navigation-mutations.json` records producer controls and explicitly excludes surviving or setup-only probes. The reproducible generator is `rust/reference-tools/users/render_ledger_navigation_mapping.py`. No Rust production bug was confirmed during harness construction; no Rails production JavaScript/CSS or Rust producer files were edited.

Fixture fidelity correction: original user fixture tour completion timestamps remain unchanged. Only the original JZ nil tour premise is applied in `header-tour`. Keyboard assertions35 and199 re-find the displayed enabled field at assertion time; assertion47 retains its original editor reference.

The starting fixture database contains no supplemental navigation rooms. Only the room creates that precede declaration sign-in run in the case fixture hook. The unread direct is created after class sign-in and before the original HQ join. Huddle configuration is scoped to the original header class; unread-badges enables it after class sign-in at its original block and restores its previous values in `finally`; other cases receive no Huddle overrides. Pure fixture pins, threads, memberships and timestamps are preserved.

Landscape driver readiness: after the original menu visibility, Stage text and bottom-boundary predicates, one rendering-frame synchronization lets the production menu finish its queued initial focus before the original page-level End action. The harness assigns no focus, mutates no DOM and keeps the original two-second focus deadline. This is a bounded driver timing adaptation; no guaranteed Selenium animation-frame contract is claimed.

Historical exact-auth negative controls: before the final null-store correction, the runner rejected 16 intended defects across 14 distinct legacy declarations, including hidden help, foreign menu scope, zero-width header actions (observed width0 at the mutation point) and an out-of-frame header. Every retained record embeds its actual exception, evidence hash, real context-disposal marker, command, forgery/reset output and execution source digests. Prior environment/cookie shortcuts and the unread geometry attempt that failed before its target are uncredited.

Final attribution requires the private Rust host to mirror pinned `config/environments/test.rb:28–29` (`perform_caching=false`, null store). The direct-row cache otherwise reuses a membership version whose frozen timestamp has not advanced when unread changes. This was a test-context mismatch, not evidence for altering the production direct-room fixture or cache key. The final controls below were rerun under the corrected host.

Final control attribution is complete: 16 intended failures across 14 distinct legacy declarations ran with real credential authentication, actual Rails test environment/forgery policy, pure fixture foundation, test null-store caching and per-case Kit rate-limit reset. All four variants reached their intended scoped/geometry assertions; each counted record embeds literal exception coordinates, actual restoration/reset output, evidence hashes, commands and execution source digests. Earlier runs remain uncredited history. Both final paired positive wrapper runs passed; the lead verified the complete logs and unchanged source digests.
