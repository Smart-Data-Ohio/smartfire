# Individual pinned original assertion audit

Current reference: `78b9b1546bdab4c6c1c9b8ddb94512f661289112`. Historical mapping: `dae59026` / `d7c7de92`, retained in JSON history.

Every original assertion is paired with the actual routed/browser predicate. Explicit `visible: :all` checks retain attached-node semantics; default selectors count rendered nodes.

## P0354: clicking a message author opens their profile card and Message lands in the DM

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_people_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/people_group_dms_test.rb:16` — `assert_selector "#profile-card-popover:not([hidden])", wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:113; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:18` — `assert_selector ".profile-card__name", text: "Jason"` | rust/reference-tools/users/original_browser_assertions.mjs:113; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:19` — `assert_selector "button", text: "Start call"` | rust/reference-tools/users/original_browser_assertions.mjs:114; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:23` — `assert_current_path room_path(rooms(:david_and_jason)), wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:117; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:24` — `assert_selector ".room--current", text: "Jason"` | rust/reference-tools/users/original_browser_assertions.mjs:118; rust/reference-tools/users/original_browser_assertions.mjs:27 |

## P0355: the profile card opens by keyboard, traps focus, and returns it on Esc

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_people_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/people_group_dms_test.rb:34` — `assert_selector "#profile-card-popover:not([hidden])", wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:123; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:35` — `assert_selector "#user_card .profile-card__name", text: "Jason"` | rust/reference-tools/users/original_browser_assertions.mjs:123; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:38` — `assert_selector "#profile-card-popover[hidden]", visible: :all, wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:125; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:39` — `assert_equal "BUTTON", page.evaluate_script("document.activeElement.tagName")` | rust/reference-tools/users/original_browser_assertions.mjs:126; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:40` — `assert_includes page.evaluate_script("document.activeElement.textContent"), "Jason"` | rust/reference-tools/users/original_browser_assertions.mjs:127; rust/reference-tools/users/original_browser_assertions.mjs:27 |

## P0356: Esc with a closed profile card stays unhandled for later listeners

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_people_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/people_group_dms_test.rb:47` — `assert_selector "#profile-card-popover[hidden]", visible: :all` | rust/reference-tools/users/original_browser_assertions.mjs:130; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:59` — `assert_not prevented, "a closed profile card must not preventDefault Escape (huddle theater owns it)"` | rust/reference-tools/users/original_browser_assertions.mjs:132; rust/reference-tools/users/original_browser_assertions.mjs:27 |

## P0357: the sidebar avatar trigger opens the profile card by keyboard

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_people_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/people_group_dms_test.rb:68` — `assert_selector "#profile-card-popover:not([hidden])", wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:137; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:69` — `assert_selector "#user_card .profile-card__name"` | rust/reference-tools/users/original_browser_assertions.mjs:137; rust/reference-tools/users/original_browser_assertions.mjs:27 |

## P0358: multi-selecting three people in the directory lands in their group DM

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_people_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/people_group_dms_test.rb:81` — `assert_selector "button", text: "Message (3)"` | rust/reference-tools/users/original_browser_assertions.mjs:142; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:85` — `assert_selector ".room--current", text: "Jason, JZ, Kevin", wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:145; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:87` — `assert_current_path room_path(room)` | rust/reference-tools/users/run_original_browser_assertions.py:90 |

## P0359: the member panel multi-select starts a huddle with exactly that set

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_people_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/people_group_dms_test.rb:97` — `assert_selector "#channel-members .member-panel__member", minimum: 3, wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:151; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:51 |
| `test/system/people_group_dms_test.rb:103` — `assert_selector "button", text: "Start huddle (2)"` | rust/reference-tools/users/original_browser_assertions.mjs:155; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:107` — `assert_selector ".room--current", text: "Jason, Kevin", wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:158; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:109` — `assert_current_path room_path(room, huddle: "start")` | rust/reference-tools/users/run_original_browser_assertions.py:90; rust/reference-tools/users/original_browser_assertions.mjs:159 |

## P0360: shift-click extends the checkbox range

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_people_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/people_group_dms_test.rb:121` — `assert_selector "button", text: "Message (4)"` | rust/reference-tools/users/original_browser_assertions.mjs:165; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:122` — `assert_selector "button", text: "Start huddle (3)"` | rust/reference-tools/users/original_browser_assertions.mjs:166; rust/reference-tools/users/original_browser_assertions.mjs:27 |

## P0361: long-press selects a row on touch

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_people_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/people_group_dms_test.rb:147` — `assert_checked_field "select_user_#{users(:jason).id}", visible: :all` | rust/reference-tools/users/original_browser_assertions.mjs:175; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:149` — `assert_selector "button", text: "Message (1)"` | rust/reference-tools/users/original_browser_assertions.mjs:176; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:151` — `assert_selector "#profile-card-popover[hidden]", visible: :all` | rust/reference-tools/users/original_browser_assertions.mjs:177; rust/reference-tools/users/original_browser_assertions.mjs:27 |

## P0362: agents are selectable for messages but excluded from huddles

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_people_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/people_group_dms_test.rb:161` — `assert_selector "button", text: "Message (1)"` | rust/reference-tools/users/original_browser_assertions.mjs:182; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:162` — `assert_selector "[data-multi-select-target='huddleButton'][disabled]", text: "Start huddle (0)"` | rust/reference-tools/users/original_browser_assertions.mjs:183; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:163` — `assert_text "Agents can't join huddles."` | rust/reference-tools/users/original_browser_assertions.mjs:184; rust/reference-tools/users/original_browser_assertions.mjs:27 |

## P0363: start huddle keeps agents in the DM but out of the call

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_people_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/people_group_dms_test.rb:175` — `assert_selector "button", text: "Message (2)"` | rust/reference-tools/users/original_browser_assertions.mjs:189; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:176` — `assert_selector "button", text: "Start huddle (1)"` | rust/reference-tools/users/original_browser_assertions.mjs:190; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:177` — `assert_text "1 agent stays in the DM but won't be rung."` | rust/reference-tools/users/original_browser_assertions.mjs:191; rust/reference-tools/users/original_browser_assertions.mjs:27 |

## P0364: the new-DM picker filters as you type with no suggestion bubble or submit button

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_picker_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/people_group_dms_test.rb:189` — `assert_no_selector "suggestion-option"` | rust/reference-tools/users/original_browser_assertions.mjs:205; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:190` — `assert_no_text "Start Ping"` | rust/reference-tools/users/original_browser_assertions.mjs:206; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:192` — `assert total > 2, "expected the full people list before filtering"` | rust/reference-tools/users/original_browser_assertions.mjs:212; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:195` — `assert_selector ".dm-picker__row:not([hidden])", count: 1, text: "Chad Puterbaugh"` | rust/reference-tools/users/original_browser_assertions.mjs:213; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:198` — `assert_selector ".dm-picker__row:not([hidden])", count: 1, text: "Chad Puterbaugh"` | rust/reference-tools/users/original_browser_assertions.mjs:213; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:201` — `assert_selector ".dm-picker__row:not([hidden])", count: 1, text: "Renée Dupont"` | rust/reference-tools/users/original_browser_assertions.mjs:213; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:204` — `assert_no_selector ".dm-picker__row:not([hidden])"` | rust/reference-tools/users/original_browser_assertions.mjs:216; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:205` — `assert_selector "[data-dm-picker-target='empty']", text: "No one matches"` | rust/reference-tools/users/original_browser_assertions.mjs:217; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:208` — `assert_selector ".dm-picker__row:not([hidden])", count: total` | rust/reference-tools/users/original_browser_assertions.mjs:218; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:209` — `assert_no_selector "[data-dm-picker-target='empty']:not([hidden])"` | rust/reference-tools/users/original_browser_assertions.mjs:219; rust/reference-tools/users/original_browser_assertions.mjs:27 |

## P0365: picker selections survive filtering and Message starts the DM

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_picker_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/people_group_dms_test.rb:223` — `assert_no_selector ".dm-picker__row:not([hidden])", text: "Chad Puterbaugh"` | rust/reference-tools/users/original_browser_assertions.mjs:224; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:226` — `assert_selector "button", text: "Message (1)"` | rust/reference-tools/users/original_browser_assertions.mjs:225; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:232` — `assert_checked_field "pick_user_#{chad.id}", visible: :all` | rust/reference-tools/users/original_browser_assertions.mjs:226; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:238` — `assert_selector ".room--current", text: "Chad", wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:228; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:240` — `assert_current_path room_path(room)` | rust/reference-tools/users/run_original_browser_assertions.py:90 |

## P0366: Enter in the picker filter selects the single visible match

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_picker_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/people_group_dms_test.rb:252` — `assert_selector ".dm-picker__row:not([hidden])", minimum: 2` | rust/reference-tools/users/original_browser_assertions.mjs:232; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:254` — `assert_no_selector "[data-multi-select-target='bar']:not([hidden])"` | rust/reference-tools/users/original_browser_assertions.mjs:233; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:260` — `assert_checked_field "pick_user_#{chad.id}", visible: :all` | rust/reference-tools/users/original_browser_assertions.mjs:234; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:267` — `assert_checked_field "pick_user_#{chad.id}", visible: :all` | rust/reference-tools/users/original_browser_assertions.mjs:236; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:268` — `assert_unchecked_field "pick_user_#{users(:kevin).id}", visible: :all` | rust/reference-tools/users/original_browser_assertions.mjs:236; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:270` — `assert_selector "button", text: "Message (1)"` | rust/reference-tools/users/original_browser_assertions.mjs:237; rust/reference-tools/users/original_browser_assertions.mjs:27 |

## P0367: clicking a picker row toggles it while the name still opens the profile card

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_picker_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/people_group_dms_test.rb:281` — `assert_checked_field "pick_user_#{users(:kevin).id}", visible: :all` | rust/reference-tools/users/original_browser_assertions.mjs:242; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:284` — `assert_selector "#profile-card-popover:not([hidden])", wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:243; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:285` — `assert_selector "#user_card .profile-card__name", text: "Kevin"` | rust/reference-tools/users/original_browser_assertions.mjs:243; rust/reference-tools/users/original_browser_assertions.mjs:27 |

## P0368: the new-DM picker does not overflow at phone width

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_picker_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/people_group_dms_test.rb:292` — `assert_selector "#sidebar.open"` | rust/reference-tools/users/original_browser_assertions.mjs:199; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:299` — `assert page.evaluate_script("document.documentElement.scrollWidth <= window.innerWidth + 1"),` | rust/reference-tools/users/original_browser_assertions.mjs:248; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:303` — `assert_operator row_height, :>=, 44, "expected full touch-target rows at phone width"` | rust/reference-tools/users/original_browser_assertions.mjs:249; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:305` — `assert_in_delta 32, avatar_width, 1, "expected 32px picker avatars"` | rust/reference-tools/users/original_browser_assertions.mjs:250; rust/reference-tools/users/original_browser_assertions.mjs:27 |

## P0369: group members rename, add, and leave with system notes in the timeline

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_group_lifecycle_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/people_group_dms_test.rb:321` — `assert_text "Group renamed."` | rust/reference-tools/users/original_browser_assertions.mjs:300; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:322` — `assert_field "Group name", with: "Weekend Plans"` | rust/reference-tools/users/original_browser_assertions.mjs:301; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:326` — `assert_selector ".directs--edit", text: "JZ"` | rust/reference-tools/users/original_browser_assertions.mjs:303; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:329` — `assert_selector ".room--current", text: "Weekend Plans"` | rust/reference-tools/users/original_browser_assertions.mjs:304; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:330` — `assert_selector ".message__system-note", text: "renamed the group to Weekend Plans"` | rust/reference-tools/users/original_browser_assertions.mjs:305; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:331` — `assert_selector ".message__system-note", text: "added JZ to the group"` | rust/reference-tools/users/original_browser_assertions.mjs:306; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:332` — `assert_text(/David\s+renamed the group to Weekend Plans/)` | rust/reference-tools/users/original_browser_assertions.mjs:307; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:333` — `assert_text(/David\s+added JZ to the group/)` | rust/reference-tools/users/original_browser_assertions.mjs:308; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:340` — `assert_no_current_path edit_rooms_direct_path(room), wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:318; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:341` — `assert_no_current_path room_path(room), wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:320; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:342` — `assert_not room.reload.user_ids.include?(users(:david).id)` | rust/reference-tools/users/run_original_browser_assertions.py:94 |

## P0371: Esc closes only the profile card inside the mobile member panel

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_mobile_member_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/people_group_dms_test.rb:372` — `assert_selector "#channel-members .member-panel__member", minimum: 3, wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:268; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:51 |
| `test/system/people_group_dms_test.rb:377` — `assert_selector "#profile-card-popover:not([hidden])", wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:271; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:381` — `assert_selector "#profile-card-popover[hidden]", visible: :all, wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:273; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:382` — `assert_selector "#channel-members", visible: true` | rust/reference-tools/users/original_browser_assertions.mjs:274; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:383` — `assert_button "Close members"` | rust/reference-tools/users/original_browser_assertions.mjs:274; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:384` — `assert_focused trigger` | rust/reference-tools/users/original_browser_assertions.mjs:274; rust/reference-tools/users/original_browser_assertions.mjs:27 |

## P0372: Tab cycles within the profile card opened from the member panel

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_mobile_member_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/people_group_dms_test.rb:396` — `assert_selector "#channel-members .member-panel__member", minimum: 3, wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:268; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:51 |
| `test/system/people_group_dms_test.rb:400` — `assert_selector "#profile-card-popover:not([hidden])", wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:271; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:401` — `assert_selector "#user_card .profile-card__name", text: "Kevin"` | rust/reference-tools/users/original_browser_assertions.mjs:276; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:406` — `assert_focused ".profile-card-popover__close"` | rust/reference-tools/users/original_browser_assertions.mjs:277; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:408` — `assert_focused "#user_card .profile-card__actions form:nth-of-type(1) button"` | rust/reference-tools/users/original_browser_assertions.mjs:278; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/people_group_dms_test.rb:410` — `assert_focused "#user_card .profile-card__actions form:nth-of-type(2) button"` | rust/reference-tools/users/original_browser_assertions.mjs:279; rust/reference-tools/users/original_browser_assertions.mjs:27 |

## P0300: a new member is walked through the tour by keyboard and finishing persists

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_tour_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/first_run_tour_test.rb:12` — `assert_selector "#tour .tour__card", visible: true` | rust/reference-tools/users/original_browser_assertions.mjs:345; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/first_run_tour_test.rb:13` — `assert_selector ".tour__progress", text: "Step 1 of 5"` | rust/reference-tools/users/original_browser_assertions.mjs:345; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:325 |
| `test/system/first_run_tour_test.rb:14` — `assert_selector ".tour__title", text: "Your rooms live here"` | rust/reference-tools/users/original_browser_assertions.mjs:345; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/first_run_tour_test.rb:15` — `assert_selector "#sidebar.tour__target"` | rust/reference-tools/users/original_browser_assertions.mjs:345; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/first_run_tour_test.rb:18` — `assert_selector ".tour__progress", text: "Step 2 of 5"` | rust/reference-tools/users/original_browser_assertions.mjs:346; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:325 |
| `test/system/first_run_tour_test.rb:19` — `assert_selector "#composer.tour__target"` | rust/reference-tools/users/original_browser_assertions.mjs:346; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/first_run_tour_test.rb:22` — `assert_selector ".tour__progress", text: "Step 3 of 5"` | rust/reference-tools/users/original_browser_assertions.mjs:347; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:325 |
| `test/system/first_run_tour_test.rb:23` — `assert_selector ".tour__card--center"` | rust/reference-tools/users/original_browser_assertions.mjs:347; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/first_run_tour_test.rb:26` — `assert_selector ".tour__progress", text: "Step 4 of 5"` | rust/reference-tools/users/original_browser_assertions.mjs:348; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:325 |
| `test/system/first_run_tour_test.rb:27` — `assert_selector ".tour__title", text: "Jump anywhere with Ctrl+K"` | rust/reference-tools/users/original_browser_assertions.mjs:348; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/first_run_tour_test.rb:30` — `assert_selector ".tour__progress", text: "Step 3 of 5"` | rust/reference-tools/users/original_browser_assertions.mjs:349; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:325 |
| `test/system/first_run_tour_test.rb:33` — `assert_selector ".tour__progress", text: "Step 5 of 5"` | rust/reference-tools/users/original_browser_assertions.mjs:349; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:325 |
| `test/system/first_run_tour_test.rb:34` — `assert_selector ".tour__title", text: "Shortcuts live under ?"` | rust/reference-tools/users/original_browser_assertions.mjs:350; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/first_run_tour_test.rb:35` — `assert_selector "#help-menu-button.tour__target"` | rust/reference-tools/users/original_browser_assertions.mjs:350; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/first_run_tour_test.rb:38` — `assert_no_selector "#tour .tour__card", visible: true` | rust/reference-tools/users/original_browser_assertions.mjs:351; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:329 |
| `test/system/first_run_tour_test.rb:39` — `assert_tour_completed` | rust/reference-tools/users/original_browser_assertions.mjs:333; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/first_run_tour_test.rb:42` — `assert_selector '#tour[data-tour-auto-start-value="false"]', visible: :all` | rust/reference-tools/users/original_browser_assertions.mjs:351; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:339 |
| `test/system/first_run_tour_test.rb:43` — `assert_no_selector "#tour .tour__card", visible: true` | rust/reference-tools/users/original_browser_assertions.mjs:351; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:340 |
| `test/system/first_run_tour_test.rb:95` — `end` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:333 |

## P0301: escape skips the tour and it never auto-starts again

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_tour_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/first_run_tour_test.rb:50` — `assert_selector "#tour .tour__card", visible: true` | rust/reference-tools/users/original_browser_assertions.mjs:354; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/first_run_tour_test.rb:53` — `assert_no_selector "#tour .tour__card", visible: true` | rust/reference-tools/users/original_browser_assertions.mjs:355; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:329 |
| `test/system/first_run_tour_test.rb:54` — `assert_tour_completed` | rust/reference-tools/users/original_browser_assertions.mjs:333; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/first_run_tour_test.rb:57` — `assert_selector '#tour[data-tour-auto-start-value="false"]', visible: :all` | rust/reference-tools/users/original_browser_assertions.mjs:355; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:339 |
| `test/system/first_run_tour_test.rb:58` — `assert_no_selector "#tour .tour__card", visible: true` | rust/reference-tools/users/original_browser_assertions.mjs:355; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:340 |
| `test/system/first_run_tour_test.rb:95` — `end` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:333 |

## P0302: the tour restarts from the help menu

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_tour_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/first_run_tour_test.rb:66` — `assert_no_selector "#tour .tour__card", visible: true` | rust/reference-tools/users/original_browser_assertions.mjs:361; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:329 |
| `test/system/first_run_tour_test.rb:67` — `assert_tour_completed` | rust/reference-tools/users/original_browser_assertions.mjs:333; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/first_run_tour_test.rb:72` — `assert_selector "#tour .tour__card", visible: true` | rust/reference-tools/users/original_browser_assertions.mjs:364; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/first_run_tour_test.rb:73` — `assert_selector ".tour__progress", text: "Step 1 of 5"` | rust/reference-tools/users/original_browser_assertions.mjs:364; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:325 |
| `test/system/first_run_tour_test.rb:95` — `end` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:333 |

## P0303: members who completed the tour never see it auto-start

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_tour_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/first_run_tour_test.rb:82` — `assert_selector '#tour[data-tour-auto-start-value="false"]', visible: :all` | rust/reference-tools/users/original_browser_assertions.mjs:373; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/first_run_tour_test.rb:83` — `assert_no_selector "#tour .tour__card", visible: true` | rust/reference-tools/users/original_browser_assertions.mjs:374; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/first_run_tour_test.rb:84` — `assert_selector "#help-menu-button", visible: true` | rust/reference-tools/users/original_browser_assertions.mjs:374; rust/reference-tools/users/original_browser_assertions.mjs:27 |

## P0400: starring from the profile card floats the person into a Starred group

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_starred_people_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/starred_people_test.rb:21` — `assert_no_selector "#channel-members [aria-label='Starred members']", visible: :all` | rust/reference-tools/users/original_browser_assertions.mjs:415; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:26` — `assert_selector "button", text: "★ Unstar", wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:417; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:29` — `assert_starred kevin` | rust/reference-tools/users/original_browser_assertions.mjs:395; rust/reference-tools/users/original_browser_assertions.mjs:396; rust/reference-tools/users/original_browser_assertions.mjs:397; rust/reference-tools/users/original_browser_assertions.mjs:398; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:30` — `assert_member_rows_inline` | rust/reference-tools/users/original_browser_assertions.mjs:409; rust/reference-tools/users/original_browser_assertions.mjs:411; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:36` — `assert_selector "button", text: "☆ Star", wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:419; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:40` — `assert_unstarred kevin` | rust/reference-tools/users/original_browser_assertions.mjs:400; rust/reference-tools/users/original_browser_assertions.mjs:401; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:41` — `assert_member_rows_inline` | rust/reference-tools/users/original_browser_assertions.mjs:409; rust/reference-tools/users/original_browser_assertions.mjs:411; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:144` — `assert_selector "#channel-members [aria-label='Starred members'] [data-member-id='#{user.id}'][data-starred='true']",` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:395 |
| `test/system/starred_people_test.rb:146` — `assert_no_selector "#channel-members [aria-label='Online members'] [data-member-id='#{user.id}']", visible: :all` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:396 |
| `test/system/starred_people_test.rb:147` — `assert_no_selector "#channel-members [aria-label='Offline members'] [data-member-id='#{user.id}']", visible: :all` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:397 |
| `test/system/starred_people_test.rb:148` — `assert_selector "#channel-members [aria-label='Starred members'] [data-member-id='#{user.id}'] .member-panel__presence",` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:398 |
| `test/system/starred_people_test.rb:168` — `assert_selector "#channel-members .member-panel__member .member-panel__identity"` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:409 |
| `test/system/starred_people_test.rb:176` — `assert_empty broken, "member rows wrapped their names under the avatar"` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:411 |
| `test/system/starred_people_test.rb:153` — `assert_no_selector "#channel-members [aria-label='Starred members']", visible: :all` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:400 |
| `test/system/starred_people_test.rb:154` — `assert_selector "#channel-members [aria-label='Offline members'] [data-member-id='#{user.id}'][data-starred='false']",` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:401 |
| `test/system/starred_people_test.rb:168` — `assert_selector "#channel-members .member-panel__member .member-panel__identity"` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:409 |
| `test/system/starred_people_test.rb:176` — `assert_empty broken, "member rows wrapped their names under the avatar"` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:411 |
| `test/system/starred_people_test.rb:134` — `assert_selector "#profile-card-popover:not([hidden])", wait: 10` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:391 |
| `test/system/starred_people_test.rb:135` — `assert_selector "#user_card .profile-card__name", text: user.name` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:391 |
| `test/system/starred_people_test.rb:140` — `assert_selector "#profile-card-popover[hidden]", visible: :all, wait: 10` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:392 |
| `test/system/starred_people_test.rb:134` — `assert_selector "#profile-card-popover:not([hidden])", wait: 10` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:391 |
| `test/system/starred_people_test.rb:135` — `assert_selector "#user_card .profile-card__name", text: user.name` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:391 |
| `test/system/starred_people_test.rb:140` — `assert_selector "#profile-card-popover[hidden]", visible: :all, wait: 10` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:392 |
| `test/system/starred_people_test.rb:11` — `assert_selector "#channel-members"` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:413 |

## P0401: the member row menu stars and unstars, by mouse and keyboard

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_starred_people_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/starred_people_test.rb:48` — `assert_selector "#member-row-menu", visible: true` | rust/reference-tools/users/original_browser_assertions.mjs:427; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:49` — `assert_focused "#member-row-menu [role='menuitem']"` | rust/reference-tools/users/original_browser_assertions.mjs:427; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:53` — `assert_selector "button", text: "★ Unstar", wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:428; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:55` — `assert_starred kevin` | rust/reference-tools/users/original_browser_assertions.mjs:395; rust/reference-tools/users/original_browser_assertions.mjs:396; rust/reference-tools/users/original_browser_assertions.mjs:397; rust/reference-tools/users/original_browser_assertions.mjs:398; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:59` — `assert_selector "button", text: "☆ Star", wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:429; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:61` — `assert_unstarred kevin` | rust/reference-tools/users/original_browser_assertions.mjs:400; rust/reference-tools/users/original_browser_assertions.mjs:401; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:64` — `assert_no_selector "#member-row-menu", visible: true` | rust/reference-tools/users/original_browser_assertions.mjs:430; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:390 |
| `test/system/starred_people_test.rb:65` — `assert_row_trigger_focused kevin` | rust/reference-tools/users/original_browser_assertions.mjs:431; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:72` — `assert_selector "#member-row-menu", visible: true` | rust/reference-tools/users/original_browser_assertions.mjs:432; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:73` — `assert_focused "#member-row-menu [role='menuitem']"` | rust/reference-tools/users/original_browser_assertions.mjs:432; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:77` — `assert_starred kevin` | rust/reference-tools/users/original_browser_assertions.mjs:395; rust/reference-tools/users/original_browser_assertions.mjs:396; rust/reference-tools/users/original_browser_assertions.mjs:397; rust/reference-tools/users/original_browser_assertions.mjs:398; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:79` — `assert_no_selector "#member-row-menu", visible: true` | rust/reference-tools/users/original_browser_assertions.mjs:433; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:390 |
| `test/system/starred_people_test.rb:83` — `assert_no_selector "#member-row-menu", visible: true` | rust/reference-tools/users/original_browser_assertions.mjs:434; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:390 |
| `test/system/starred_people_test.rb:85` — `assert_member_rows_inline` | rust/reference-tools/users/original_browser_assertions.mjs:409; rust/reference-tools/users/original_browser_assertions.mjs:411; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:144` — `assert_selector "#channel-members [aria-label='Starred members'] [data-member-id='#{user.id}'][data-starred='true']",` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:395 |
| `test/system/starred_people_test.rb:146` — `assert_no_selector "#channel-members [aria-label='Online members'] [data-member-id='#{user.id}']", visible: :all` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:396 |
| `test/system/starred_people_test.rb:147` — `assert_no_selector "#channel-members [aria-label='Offline members'] [data-member-id='#{user.id}']", visible: :all` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:397 |
| `test/system/starred_people_test.rb:148` — `assert_selector "#channel-members [aria-label='Starred members'] [data-member-id='#{user.id}'] .member-panel__presence",` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:398 |
| `test/system/starred_people_test.rb:153` — `assert_no_selector "#channel-members [aria-label='Starred members']", visible: :all` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:400 |
| `test/system/starred_people_test.rb:154` — `assert_selector "#channel-members [aria-label='Offline members'] [data-member-id='#{user.id}'][data-starred='false']",` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:401 |
| `test/system/starred_people_test.rb:159` — `assert page.evaluate_script(<<~JS, user.id), "expected focus to return to the member row"` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:431 |
| `test/system/starred_people_test.rb:144` — `assert_selector "#channel-members [aria-label='Starred members'] [data-member-id='#{user.id}'][data-starred='true']",` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:395 |
| `test/system/starred_people_test.rb:146` — `assert_no_selector "#channel-members [aria-label='Online members'] [data-member-id='#{user.id}']", visible: :all` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:396 |
| `test/system/starred_people_test.rb:147` — `assert_no_selector "#channel-members [aria-label='Offline members'] [data-member-id='#{user.id}']", visible: :all` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:397 |
| `test/system/starred_people_test.rb:148` — `assert_selector "#channel-members [aria-label='Starred members'] [data-member-id='#{user.id}'] .member-panel__presence",` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:398 |
| `test/system/starred_people_test.rb:168` — `assert_selector "#channel-members .member-panel__member .member-panel__identity"` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:409 |
| `test/system/starred_people_test.rb:176` — `assert_empty broken, "member rows wrapped their names under the avatar"` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:411 |
| `test/system/starred_people_test.rb:11` — `assert_selector "#channel-members"` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:413 |

## P0402: Escape still dismisses the row menu after the profile card takes focus

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_starred_people_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/starred_people_test.rb:93` — `assert_selector "#member-row-menu", visible: true` | rust/reference-tools/users/original_browser_assertions.mjs:437; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:100` — `assert_selector "#profile-card-popover:not([hidden])", wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:438; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:101` — `assert_selector "#member-row-menu:not([hidden])", visible: :all` | rust/reference-tools/users/original_browser_assertions.mjs:438; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:105` — `assert_selector "#profile-card-popover[hidden]", visible: :all, wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:440; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:106` — `assert_no_selector "#member-row-menu", visible: true` | rust/reference-tools/users/original_browser_assertions.mjs:440; rust/reference-tools/users/original_browser_assertions.mjs:27; rust/reference-tools/users/original_browser_assertions.mjs:390 |
| `test/system/starred_people_test.rb:11` — `assert_selector "#channel-members"` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:413 |

## P0403: the Starred group works on a phone

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_starred_people_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/system/starred_people_test.rb:112` — `assert_no_selector "#channel-members", visible: true` | rust/reference-tools/users/original_browser_assertions.mjs:444; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:115` — `assert_selector "#channel-members [data-member-id='#{kevin.id}']", wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:445; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:120` — `assert_selector "button", text: "★ Unstar", wait: 10` | rust/reference-tools/users/original_browser_assertions.mjs:447; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:124` — `assert_starred kevin` | rust/reference-tools/users/original_browser_assertions.mjs:395; rust/reference-tools/users/original_browser_assertions.mjs:396; rust/reference-tools/users/original_browser_assertions.mjs:397; rust/reference-tools/users/original_browser_assertions.mjs:398; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:125` — `assert_member_rows_inline` | rust/reference-tools/users/original_browser_assertions.mjs:409; rust/reference-tools/users/original_browser_assertions.mjs:411; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:126` — `assert_no_horizontal_overflow` | rust/reference-tools/users/original_browser_assertions.mjs:448; rust/reference-tools/users/original_browser_assertions.mjs:27 |
| `test/system/starred_people_test.rb:144` — `assert_selector "#channel-members [aria-label='Starred members'] [data-member-id='#{user.id}'][data-starred='true']",` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:395 |
| `test/system/starred_people_test.rb:146` — `assert_no_selector "#channel-members [aria-label='Online members'] [data-member-id='#{user.id}']", visible: :all` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:396 |
| `test/system/starred_people_test.rb:147` — `assert_no_selector "#channel-members [aria-label='Offline members'] [data-member-id='#{user.id}']", visible: :all` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:397 |
| `test/system/starred_people_test.rb:148` — `assert_selector "#channel-members [aria-label='Starred members'] [data-member-id='#{user.id}'] .member-panel__presence",` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:398 |
| `test/system/starred_people_test.rb:168` — `assert_selector "#channel-members .member-panel__member .member-panel__identity"` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:409 |
| `test/system/starred_people_test.rb:176` — `assert_empty broken, "member rows wrapped their names under the avatar"` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:411 |
| `test/system/starred_people_test.rb:180` — `assert page.evaluate_script("document.documentElement.scrollWidth <= innerWidth + 1"), "the workspace overflows the viewport horizontally"` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:448 |
| `test/system/starred_people_test.rb:134` — `assert_selector "#profile-card-popover:not([hidden])", wait: 10` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:391 |
| `test/system/starred_people_test.rb:135` — `assert_selector "#user_card .profile-card__name", text: user.name` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:391 |
| `test/system/starred_people_test.rb:140` — `assert_selector "#profile-card-popover[hidden]", visible: :all, wait: 10` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:392 |
| `test/system/starred_people_test.rb:11` — `assert_selector "#channel-members"` (helper/setup) | rust/reference-tools/users/original_browser_assertions.mjs:413 |

## P0276: service worker fetch and notification logic

Status: **closed**. Executed identities:

- `controllers::ws11ui_original_browser_tests::original_node_event_harness_assertions`

| Rails assertion | Discriminating Rust assertion |
| --- | --- |
| `test/controllers/pwa_controller_test.rb:58` — `assert $?.success?, output` | rust/reference-tools/users/original_worker_assertions.mjs:18 |
| `test/controllers/pwa_controller_test.rb:59` — `assert_includes output, "all checks passed"` | rust/reference-tools/users/original_worker_assertions.mjs:20 |
