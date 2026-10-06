# Pinned member, channel and motion assertion map

Reference: `78b9b1546bdab4c6c1c9b8ddb94512f661289112`.

Source map: 31 declarations, 213 direct assertions, 43 helper assertions, 55 setup assertions, and 234 synchronized/observation/fixture expansions. Each expansion cites its actual original source file.

Every sign-in now executes the original credential GET `/test_session`, which verifies the password and creates a real verified session. Final credential-path/null-cache controls rejected 15 defects across 11 distinct closures, with zero invalid controls and all producers restored. The registered paired wrapper passed twice at nextest -j 4, under default and CI profiles.

## P0322: member rows render without checkboxes and stay inline on desktop and phone

Case: `member-inline`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/member_select_mode_test.rb:19` — `assert_no_selector "#channel-members input[type='checkbox']"` | `rust/reference-tools/users/ledger_browser_members.mjs:294` |
| `test/system/member_select_mode_test.rb:20` — `assert_selector "#channel-members input[type='checkbox']", visible: :all, count: 3` | `rust/reference-tools/users/ledger_browser_members.mjs:295` |
| `test/system/member_select_mode_test.rb:21` — `assert_no_selector "#channel-members [data-member-id='#{users(:david).id}'] input[type='checkbox']", visible: :all` | `rust/reference-tools/users/ledger_browser_members.mjs:296` |
| `test/system/member_select_mode_test.rb:22` — `assert_no_selector "#channel-members [data-multi-select-target='bar']"` | `rust/reference-tools/users/ledger_browser_members.mjs:297` |
| `test/system/member_select_mode_test.rb:23` — `assert_selector "#channel-members [data-multi-select-target='bar']:not([aria-live])", visible: :all` | `rust/reference-tools/users/ledger_browser_members.mjs:298` |
| `test/system/member_select_mode_test.rb:24` — `assert_selector "#channel-members [data-multi-select-target='status']", text: "0 selected"` | `rust/reference-tools/users/ledger_browser_members.mjs:299` |
| `test/system/member_select_mode_test.rb:25` — `assert_member_rows_inline` | `rust/reference-tools/users/ledger_browser_members.mjs:301` |
| `test/system/member_select_mode_test.rb:28` — `assert_no_selector "#channel-members", visible: true` | `rust/reference-tools/users/ledger_browser_members.mjs:303` |
| `test/system/member_select_mode_test.rb:30` — `assert_selector "#channel-members .member-panel__member", minimum: 3, wait: 10` | `rust/reference-tools/users/ledger_browser_members.mjs:305` |
| `test/system/member_select_mode_test.rb:32` — `assert_no_selector "#channel-members input[type='checkbox']"` | `rust/reference-tools/users/ledger_browser_members.mjs:306` |
| `test/system/member_select_mode_test.rb:33` — `assert_member_rows_inline` | `rust/reference-tools/users/ledger_browser_members.mjs:307` |

## P0323: a plain click opens the profile card outside selection mode and toggles inside it

Case: `member-plain`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/member_select_mode_test.rb:40` — `assert_selector "#profile-card-popover:not([hidden])", wait: 10` | `rust/reference-tools/users/ledger_browser_members.mjs:311` |
| `test/system/member_select_mode_test.rb:41` — `assert_selector "#user_card .profile-card__name", text: "Jason"` | `rust/reference-tools/users/ledger_browser_members.mjs:311` |
| `test/system/member_select_mode_test.rb:43` — `assert_selector "#profile-card-popover[hidden]", visible: :all, wait: 10` | `rust/reference-tools/users/ledger_browser_members.mjs:312` |
| `test/system/member_select_mode_test.rb:46` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:314` |
| `test/system/member_select_mode_test.rb:47` — `assert_selector "#profile-card-popover[hidden]", visible: :all` | `rust/reference-tools/users/ledger_browser_members.mjs:314` |
| `test/system/member_select_mode_test.rb:50` — `within_bar { assert_selector "button", text: "Message (2)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:315` |
| `test/system/member_select_mode_test.rb:51` — `assert_selector "#profile-card-popover[hidden]", visible: :all` | `rust/reference-tools/users/ledger_browser_members.mjs:315` |
| `test/system/member_select_mode_test.rb:54` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:316` |

## P0324: ctrl-click and cmd-click toggle members, and dropping to zero exits the mode

Case: `member-modifiers`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/member_select_mode_test.rb:59` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:319` |
| `test/system/member_select_mode_test.rb:60` — `assert_selector "#channel-members input[type='checkbox']", count: 3` | `rust/reference-tools/users/ledger_browser_members.mjs:321` |
| `test/system/member_select_mode_test.rb:61` — `assert_member_rows_inline` | `rust/reference-tools/users/ledger_browser_members.mjs:321` |
| `test/system/member_select_mode_test.rb:62` — `assert_selector "#channel-members input[aria-label='Select Jason']"` | `rust/reference-tools/users/ledger_browser_members.mjs:322` |
| `test/system/member_select_mode_test.rb:63` — `assert_selector "#profile-card-popover[hidden]", visible: :all` | `rust/reference-tools/users/ledger_browser_members.mjs:322` |
| `test/system/member_select_mode_test.rb:66` — `assert_no_selector "#channel-members [data-multi-select-target='bar']"` | `rust/reference-tools/users/ledger_browser_members.mjs:323` |
| `test/system/member_select_mode_test.rb:67` — `assert_no_selector "#channel-members input[type='checkbox']"` | `rust/reference-tools/users/ledger_browser_members.mjs:323` |
| `test/system/member_select_mode_test.rb:70` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:325` |
| `test/system/member_select_mode_test.rb:71` — `assert_selector "#profile-card-popover[hidden]", visible: :all` | `rust/reference-tools/users/ledger_browser_members.mjs:325` |
| `test/system/member_select_mode_test.rb:74` — `within_bar { assert_selector "button", text: "Message (2)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:326` |

## P0325: ctrl-shift-click selects a range across groups and skips your own row

Case: `member-range`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/member_select_mode_test.rb:80` — `assert_selector "#channel-members [aria-label='Starred members'] [data-member-id='#{users(:jason).id}']", visible: :all, wait: 10` | `rust/reference-tools/users/ledger_browser_members.mjs:331` |
| `test/system/member_select_mode_test.rb:81` — `assert_selector "#channel-members [data-member-id='#{users(:david).id}'][data-online='true']", visible: :all, wait: 20` | `rust/reference-tools/users/ledger_browser_members.mjs:332` |
| `test/system/member_select_mode_test.rb:82` — `assert_equal [ users(:jason).id, users(:david).id, users(:jz).id, users(:kevin).id ], visual_member_order` | `rust/reference-tools/users/ledger_browser_members.mjs:333` |
| `test/system/member_select_mode_test.rb:85` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:334` |
| `test/system/member_select_mode_test.rb:88` — `within_bar { assert_selector "button", text: "Message (2)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:334` |
| `test/system/member_select_mode_test.rb:89` — `assert_checked_field "select-member-#{users(:jason).id}"` | `rust/reference-tools/users/ledger_browser_members.mjs:335` |
| `test/system/member_select_mode_test.rb:90` — `assert_checked_field "select-member-#{users(:jz).id}"` | `rust/reference-tools/users/ledger_browser_members.mjs:335` |
| `test/system/member_select_mode_test.rb:95` — `within_bar { assert_selector "button", text: "Message (3)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:336` |
| `test/system/member_select_mode_test.rb:97` — `within_bar { assert_selector "button", text: "Message (2)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:336` |
| `test/system/member_select_mode_test.rb:99` — `within_bar { assert_selector "button", text: "Message (3)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:337` |
| `test/system/member_select_mode_test.rb:100` — `assert_checked_field "select-member-#{users(:jason).id}"` | `rust/reference-tools/users/ledger_browser_members.mjs:338` |
| `test/system/member_select_mode_test.rb:101` — `assert_checked_field "select-member-#{users(:jz).id}"` | `rust/reference-tools/users/ledger_browser_members.mjs:338` |
| `test/system/member_select_mode_test.rb:102` — `assert_checked_field "select-member-#{users(:kevin).id}"` | `rust/reference-tools/users/ledger_browser_members.mjs:338` |

## P0326: ctrl-shift-click on a checked checkbox adds the range instead of clearing it

Case: `member-range-box`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/member_select_mode_test.rb:107` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:341` |
| `test/system/member_select_mode_test.rb:112` — `within_bar { assert_selector "button", text: "Message (2)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:341` |
| `test/system/member_select_mode_test.rb:116` — `within_bar { assert_selector "button", text: "Message (3)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:342` |
| `test/system/member_select_mode_test.rb:117` — `assert_checked_field "select-member-#{users(:jason).id}"` | `rust/reference-tools/users/ledger_browser_members.mjs:343` |
| `test/system/member_select_mode_test.rb:118` — `assert_checked_field "select-member-#{users(:jz).id}"` | `rust/reference-tools/users/ledger_browser_members.mjs:343` |
| `test/system/member_select_mode_test.rb:119` — `assert_checked_field "select-member-#{users(:kevin).id}"` | `rust/reference-tools/users/ledger_browser_members.mjs:343` |

## P0327: a plain click opens the profile card again after the selected member leaves

Case: `member-departed-profile`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/member_select_mode_test.rb:124` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:346` |
| `test/system/member_select_mode_test.rb:128` — `assert_no_selector "#channel-members [data-member-id='#{users(:jason).id}']", visible: :all` | `rust/reference-tools/users/ledger_browser_members.mjs:347` |
| `test/system/member_select_mode_test.rb:129` — `assert_no_selector "#channel-members [data-multi-select-target='bar']"` | `rust/reference-tools/users/ledger_browser_members.mjs:347` |
| `test/system/member_select_mode_test.rb:132` — `assert_selector "#profile-card-popover:not([hidden])", wait: 10` | `rust/reference-tools/users/ledger_browser_members.mjs:348` |
| `test/system/member_select_mode_test.rb:133` — `assert_selector "#user_card .profile-card__name", text: "Kevin"` | `rust/reference-tools/users/ledger_browser_members.mjs:348` |

## P0328: esc and the exit button leave selection mode with the panel open

Case: `member-exits`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/member_select_mode_test.rb:138` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:351` |
| `test/system/member_select_mode_test.rb:141` — `assert_no_selector "#channel-members [data-multi-select-target='bar']"` | `rust/reference-tools/users/ledger_browser_members.mjs:352` |
| `test/system/member_select_mode_test.rb:142` — `assert_no_selector "#channel-members input[type='checkbox']"` | `rust/reference-tools/users/ledger_browser_members.mjs:352` |
| `test/system/member_select_mode_test.rb:143` — `assert_selector "#channel-members", visible: true` | `rust/reference-tools/users/ledger_browser_members.mjs:352` |
| `test/system/member_select_mode_test.rb:146` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:353` |
| `test/system/member_select_mode_test.rb:148` — `assert_no_selector "#channel-members [data-multi-select-target='bar']"` | `rust/reference-tools/users/ledger_browser_members.mjs:354` |
| `test/system/member_select_mode_test.rb:149` — `assert_no_selector "#channel-members input[type='checkbox']"` | `rust/reference-tools/users/ledger_browser_members.mjs:354` |
| `test/system/member_select_mode_test.rb:150` — `assert_selector "#channel-members", visible: true` | `rust/reference-tools/users/ledger_browser_members.mjs:354` |

## P0329: the exit button returns focus to the last-touched row

Case: `member-exit-focus`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/member_select_mode_test.rb:155` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:357` |
| `test/system/member_select_mode_test.rb:158` — `assert_no_selector "#channel-members [data-multi-select-target='bar']"` | `rust/reference-tools/users/ledger_browser_members.mjs:358` |
| `test/system/member_select_mode_test.rb:159` — `assert_focused_row_name(users(:jason))` | `rust/reference-tools/users/ledger_browser_members.mjs:358` |

## P0330: the exit button falls back to the first row when the last-touched row is gone

Case: `member-fallback-focus`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/member_select_mode_test.rb:165` — `within_bar { assert_selector "button", text: "Message (2)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:361` |
| `test/system/member_select_mode_test.rb:169` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:361` |
| `test/system/member_select_mode_test.rb:172` — `assert_no_selector "#channel-members [data-multi-select-target='bar']"` | `rust/reference-tools/users/ledger_browser_members.mjs:362` |
| `test/system/member_select_mode_test.rb:173` — `assert_focused_first_row_name` | `rust/reference-tools/users/ledger_browser_members.mjs:362` |

## P0331: unchecking the last box returns focus to that row

Case: `member-uncheck-focus`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/member_select_mode_test.rb:178` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:365` |
| `test/system/member_select_mode_test.rb:181` — `assert_no_selector "#channel-members [data-multi-select-target='bar']"` | `rust/reference-tools/users/ledger_browser_members.mjs:365` |
| `test/system/member_select_mode_test.rb:182` — `assert_focused_row_name(users(:jason))` | `rust/reference-tools/users/ledger_browser_members.mjs:365` |

## P0332: esc closes the mobile drawer when only a departed member's hidden selection remains

Case: `member-mobile-departed`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/member_select_mode_test.rb:187` — `assert_no_selector "#channel-members", visible: true` | `rust/reference-tools/users/ledger_browser_members.mjs:368` |
| `test/system/member_select_mode_test.rb:189` — `assert_selector "#channel-members .member-panel__member", minimum: 3, wait: 10` | `rust/reference-tools/users/ledger_browser_members.mjs:369` |
| `test/system/member_select_mode_test.rb:193` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:370` |
| `test/system/member_select_mode_test.rb:197` — `assert_no_selector "#channel-members [data-member-id='#{users(:jason).id}']", visible: :all` | `rust/reference-tools/users/ledger_browser_members.mjs:371` |
| `test/system/member_select_mode_test.rb:198` — `assert_no_selector "#channel-members [data-multi-select-target='bar']"` | `rust/reference-tools/users/ledger_browser_members.mjs:371` |
| `test/system/member_select_mode_test.rb:201` — `assert_no_selector "#channel-members", visible: true` | `rust/reference-tools/users/ledger_browser_members.mjs:372` |

## P0333: esc on desktop leaves a departed member's hidden selection alone

Case: `member-desktop-departed`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/member_select_mode_test.rb:208` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:375` |
| `test/system/member_select_mode_test.rb:212` — `assert_no_selector "#channel-members [data-multi-select-target='bar']"` | `rust/reference-tools/users/ledger_browser_members.mjs:375` |
| `test/system/member_select_mode_test.rb:215` — `assert_selector "#channel-members", visible: true` | `rust/reference-tools/users/ledger_browser_members.mjs:376` |
| `test/system/member_select_mode_test.rb:219` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:376` |
| `test/system/member_select_mode_test.rb:220` — `assert_checked_field "select-member-#{users(:jason).id}"` | `rust/reference-tools/users/ledger_browser_members.mjs:376` |

## P0334: esc with the row menu open closes only the menu

Case: `member-menu-escape`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/member_select_mode_test.rb:225` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:379` |
| `test/system/member_select_mode_test.rb:228` — `assert_selector "#member-row-menu", visible: true` | `rust/reference-tools/users/ledger_browser_members.mjs:379` |
| `test/system/member_select_mode_test.rb:235` — `assert_no_selector "#member-row-menu"` | `rust/reference-tools/users/ledger_browser_members.mjs:380` |
| `test/system/member_select_mode_test.rb:236` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:380` |

## P0335: the selection count is announced through a dedicated status element

Case: `member-status`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/member_select_mode_test.rb:240` — `assert_selector "#channel-members [data-multi-select-target='status']", text: "0 selected"` | `rust/reference-tools/users/ledger_browser_members.mjs:384` |
| `test/system/member_select_mode_test.rb:243` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:384` |
| `test/system/member_select_mode_test.rb:244` — `assert_selector "#channel-members [data-multi-select-target='status']", text: "1 selected"` | `rust/reference-tools/users/ledger_browser_members.mjs:384` |
| `test/system/member_select_mode_test.rb:247` — `within_bar { assert_selector "button", text: "Message (2)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:385` |
| `test/system/member_select_mode_test.rb:248` — `assert_selector "#channel-members [data-multi-select-target='status']", text: "2 selected"` | `rust/reference-tools/users/ledger_browser_members.mjs:385` |
| `test/system/member_select_mode_test.rb:251` — `assert_no_selector "#channel-members [data-multi-select-target='bar']"` | `rust/reference-tools/users/ledger_browser_members.mjs:386` |
| `test/system/member_select_mode_test.rb:252` — `assert_selector "#channel-members [data-multi-select-target='status']", text: "0 selected"` | `rust/reference-tools/users/ledger_browser_members.mjs:386` |

## P0336: space toggles the focused row without opening the profile card

Case: `member-space`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/member_select_mode_test.rb:260` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:389` |
| `test/system/member_select_mode_test.rb:261` — `assert_selector "#profile-card-popover[hidden]", visible: :all` | `rust/reference-tools/users/ledger_browser_members.mjs:389` |
| `test/system/member_select_mode_test.rb:262` — `assert_checked_field "select-member-#{users(:jason).id}"` | `rust/reference-tools/users/ledger_browser_members.mjs:389` |
| `test/system/member_select_mode_test.rb:265` — `assert_no_selector "#channel-members [data-multi-select-target='bar']"` | `rust/reference-tools/users/ledger_browser_members.mjs:390` |
| `test/system/member_select_mode_test.rb:266` — `assert_selector "#profile-card-popover[hidden]", visible: :all` | `rust/reference-tools/users/ledger_browser_members.mjs:390` |

## P0337: selection, mode, and anchor survive a presence re-render

Case: `member-rerender`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/member_select_mode_test.rb:271` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:393` |
| `test/system/member_select_mode_test.rb:275` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:394` |
| `test/system/member_select_mode_test.rb:276` — `assert_checked_field "select-member-#{users(:jason).id}"` | `rust/reference-tools/users/ledger_browser_members.mjs:394` |
| `test/system/member_select_mode_test.rb:281` — `within_bar { assert_selector "button", text: "Message (3)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:394` |

## P0338: the star row menu works in and out of selection mode

Case: `member-star-menu`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/member_select_mode_test.rb:286` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:397` |
| `test/system/member_select_mode_test.rb:289` — `assert_selector "#member-row-menu", visible: true` | `rust/reference-tools/users/ledger_browser_members.mjs:397` |
| `test/system/member_select_mode_test.rb:292` — `assert_selector "button", text: "★ Unstar", wait: 10` | `rust/reference-tools/users/ledger_browser_members.mjs:398` |
| `test/system/member_select_mode_test.rb:294` — `assert_selector "#channel-members [aria-label='Starred members'] [data-member-id='#{users(:kevin).id}']", visible: :all, wait: 10` | `rust/reference-tools/users/ledger_browser_members.mjs:399` |
| `test/system/member_select_mode_test.rb:295` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:399` |
| `test/system/member_select_mode_test.rb:298` — `assert_no_selector "#member-row-menu"` | `rust/reference-tools/users/ledger_browser_members.mjs:400` |
| `test/system/member_select_mode_test.rb:300` — `assert_no_selector "#channel-members [data-multi-select-target='bar']"` | `rust/reference-tools/users/ledger_browser_members.mjs:401` |
| `test/system/member_select_mode_test.rb:303` — `assert_selector "#member-row-menu", visible: true` | `rust/reference-tools/users/ledger_browser_members.mjs:402` |
| `test/system/member_select_mode_test.rb:306` — `assert_selector "button", text: "☆ Star", wait: 10` | `rust/reference-tools/users/ledger_browser_members.mjs:403` |
| `test/system/member_select_mode_test.rb:308` — `assert_no_selector "#channel-members [aria-label='Starred members']", visible: :all` | `rust/reference-tools/users/ledger_browser_members.mjs:404` |

## P0339: a phone long-press enters selection mode, then taps toggle and Message submits the set

Case: `member-long-press`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/member_select_mode_test.rb:313` — `assert_no_selector "#channel-members", visible: true` | `rust/reference-tools/users/ledger_browser_members.mjs:407` |
| `test/system/member_select_mode_test.rb:315` — `assert_selector "#channel-members .member-panel__member", minimum: 3, wait: 10` | `rust/reference-tools/users/ledger_browser_members.mjs:408` |
| `test/system/member_select_mode_test.rb:333` — `assert_no_selector "#member-row-menu", wait: 0` | `rust/reference-tools/users/ledger_browser_members.mjs:426` |
| `test/system/member_select_mode_test.rb:344` — `assert_checked_field "select-member-#{users(:jason).id}"` | `rust/reference-tools/users/ledger_browser_members.mjs:427` |
| `test/system/member_select_mode_test.rb:345` — `within_bar { assert_selector "button", text: "Message (1)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:427` |
| `test/system/member_select_mode_test.rb:346` — `assert_selector "#profile-card-popover[hidden]", visible: :all` | `rust/reference-tools/users/ledger_browser_members.mjs:427` |
| `test/system/member_select_mode_test.rb:347` — `assert_no_selector "#member-row-menu"` | `rust/reference-tools/users/ledger_browser_members.mjs:427` |
| `test/system/member_select_mode_test.rb:348` — `assert_member_rows_inline` | `rust/reference-tools/users/ledger_browser_members.mjs:427` |
| `test/system/member_select_mode_test.rb:351` — `within_bar { assert_selector "button", text: "Message (2)" }` | `rust/reference-tools/users/ledger_browser_members.mjs:428` |
| `test/system/member_select_mode_test.rb:352` — `assert_selector "#profile-card-popover[hidden]", visible: :all` | `rust/reference-tools/users/ledger_browser_members.mjs:428` |
| `test/system/member_select_mode_test.rb:356` — `assert_selector ".room--current", text: "Jason, Kevin", wait: 10` | `rust/reference-tools/users/ledger_browser_members.mjs:429` |
| `test/system/member_select_mode_test.rb:358` — `assert_current_path room_path(room)` | `rust/reference-tools/users/ledger_browser_members.mjs:432` |

## P0289: members are online across channels and go offline after signing out

Case: `channel-online`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/channel_members_test.rb:23` — `assert_member users(:jz), online: true` | `rust/reference-tools/users/ledger_browser_members.mjs:436` |
| `test/system/channel_members_test.rb:24` — `assert_member users(:kevin), online: false` | `rust/reference-tools/users/ledger_browser_members.mjs:436` |
| `test/system/channel_members_test.rb:25` — `assert_no_selector "#channel-members [data-member-id='#{users(:bender).id}']", visible: :all` | `rust/reference-tools/users/ledger_browser_members.mjs:437` |
| `test/system/channel_members_test.rb:33` — `assert_member users(:kevin), online: true` | `rust/reference-tools/users/ledger_browser_members.mjs:441` |
| `test/system/channel_members_test.rb:34` — `assert memberships(:kevin_designers).reload.unread?, "workspace presence must not mark another channel as read"` | `rust/reference-tools/users/ledger_browser_members.mjs:442` |
| `test/system/channel_members_test.rb:35` — `assert_composer_alignment` | `rust/reference-tools/users/ledger_browser_members.mjs:444` |
| `test/system/channel_members_test.rb:43` — `assert_field "email_address"` | `rust/reference-tools/users/ledger_browser_members.mjs:447` |
| `test/system/channel_members_test.rb:46` — `assert_member users(:kevin), online: false` | `rust/reference-tools/users/ledger_browser_members.mjs:448` |
| `test/system/channel_members_test.rb:47` — `assert_member users(:jz), online: true` | `rust/reference-tools/users/ledger_browser_members.mjs:448` |

## P0290: a bot with a checked-in agent shows online in the member panel

Case: `channel-agent`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/channel_members_test.rb:58` — `assert_member bot, online: true` | `rust/reference-tools/users/ledger_browser_members.mjs:452` |

## P0291: closing one tab keeps a member online until their last tab closes

Case: `channel-tabs`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/channel_members_test.rb:75` — `assert_member users(:kevin), online: true` | `rust/reference-tools/users/ledger_browser_members.mjs:458` |
| `test/system/channel_members_test.rb:79` — `assert_member users(:kevin), online: false` | `rust/reference-tools/users/ledger_browser_members.mjs:459` |
| `test/system/channel_members_test.rb:80` — `assert users(:kevin).sessions.exists?, "closing the app should not need to end the login session"` | `rust/reference-tools/users/ledger_browser_members.mjs:460` |

## P0292: the member panel follows channel access and remains usable on a phone

Case: `channel-phone`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/channel_members_test.rb:86` — `assert_member users(:bender), online: false` | `rust/reference-tools/users/ledger_browser_members.mjs:465` |
| `test/system/channel_members_test.rb:87` — `assert_no_selector "#channel-members [data-member-id='#{users(:kevin).id}']", visible: :all` | `rust/reference-tools/users/ledger_browser_members.mjs:466` |
| `test/system/channel_members_test.rb:88` — `assert_no_selector "#channel-members [data-member-id='#{users(:david).id}']", visible: :all` | `rust/reference-tools/users/ledger_browser_members.mjs:466` |
| `test/system/channel_members_test.rb:91` — `assert_no_selector "#channel-members", visible: true` | `rust/reference-tools/users/ledger_browser_members.mjs:467` |
| `test/system/channel_members_test.rb:93` — `assert_selector "#channel-members"` | `rust/reference-tools/users/ledger_browser_members.mjs:468` |
| `test/system/channel_members_test.rb:94` — `assert_member_rows_inline` | `rust/reference-tools/users/ledger_browser_members.mjs:468` |
| `test/system/channel_members_test.rb:97` — `assert_no_selector "#channel-members", visible: true` | `rust/reference-tools/users/ledger_browser_members.mjs:469` |
| `test/system/channel_members_test.rb:98` — `assert_no_horizontal_overflow` | `rust/reference-tools/users/ledger_browser_members.mjs:469` |
| `test/system/channel_members_test.rb:99` — `assert_composer_alignment` | `rust/reference-tools/users/ledger_browser_members.mjs:470` |
| `test/system/channel_members_test.rb:101` — `assert_button "Close members"` | `rust/reference-tools/users/ledger_browser_members.mjs:471` |
| `test/system/channel_members_test.rb:102` — `assert_member users(:bender), online: false` | `rust/reference-tools/users/ledger_browser_members.mjs:472` |
| `test/system/channel_members_test.rb:103` — `assert_focused "button[aria-label='Close members']"` | `rust/reference-tools/users/ledger_browser_members.mjs:472` |
| `test/system/channel_members_test.rb:105` — `assert_focused "#channel-members *"` | `rust/reference-tools/users/ledger_browser_members.mjs:473` |
| `test/system/channel_members_test.rb:107` — `assert_no_selector "#channel-members", visible: true` | `rust/reference-tools/users/ledger_browser_members.mjs:474` |
| `test/system/channel_members_test.rb:108` — `assert_focused "button[aria-label='Show members']"` | `rust/reference-tools/users/ledger_browser_members.mjs:474` |
| `test/system/channel_members_test.rb:112` — `assert_member_rows_inline` | `rust/reference-tools/users/ledger_browser_members.mjs:475` |
| `test/system/channel_members_test.rb:116` — `assert_message_text "Still easy to chat on a phone."` | `rust/reference-tools/users/ledger_browser_members.mjs:478` |
| `test/system/channel_members_test.rb:117` — `assert_composer_alignment` | `rust/reference-tools/users/ledger_browser_members.mjs:479` |
| `test/system/channel_members_test.rb:120` — `assert_no_horizontal_overflow` | `rust/reference-tools/users/ledger_browser_members.mjs:480` |
| `test/system/channel_members_test.rb:121` — `assert_button "Show members"` | `rust/reference-tools/users/ledger_browser_members.mjs:480` |
| `test/system/channel_members_test.rb:122` — `assert_composer_alignment` | `rust/reference-tools/users/ledger_browser_members.mjs:480` |

## P0345: motion is off by default in the test environment

Case: `motion-default`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/motion_test.rb:20` — `assert_equal "off", page.evaluate_script("document.documentElement.dataset.testMotion")` | `rust/reference-tools/users/ledger_browser_members.mjs:493` |
| `test/system/motion_test.rb:21` — `assert_equal "0ms", motion_token("--motion-medium")` | `rust/reference-tools/users/ledger_browser_members.mjs:494` |
| `test/system/motion_test.rb:22` — `assert_equal "0ms", motion_token("--motion-fast")` | `rust/reference-tools/users/ledger_browser_members.mjs:494` |
| `test/system/motion_test.rb:23` — `assert_equal "0ms", motion_token("--motion-quick")` | `rust/reference-tools/users/ledger_browser_members.mjs:494` |

## P0346: mobile drawer animates in, lands in place, and returns focus with motion on

Case: `motion-drawer`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/motion_test.rb:37` — `assert_selector "#sidebar.open"` | `rust/reference-tools/users/ledger_browser_members.mjs:500` |
| `test/system/motion_test.rb:43` — `assert start_tx < -1, "expected the drawer to start off-canvas, got tx=#{start_tx}"` | `rust/reference-tools/users/ledger_browser_members.mjs:501` |
| `test/system/motion_test.rb:49` — `assert mid_tx < -1, "expected a mid-travel sample, the drawer already landed (tx=#{mid_tx})"` | `rust/reference-tools/users/ledger_browser_members.mjs:503` |
| `test/system/motion_test.rb:51` — `assert_running_transition "#sidebar .sidebar__container", "transform"` | `rust/reference-tools/users/ledger_browser_members.mjs:505` |
| `test/system/motion_test.rb:61` — `assert_equal "1", page.evaluate_script("getComputedStyle(document.querySelector('#sidebar')).opacity")` | `rust/reference-tools/users/ledger_browser_members.mjs:509` |
| `test/system/motion_test.rb:67` — `assert_no_selector "#sidebar.open"` | `rust/reference-tools/users/ledger_browser_members.mjs:511` |

## P0347: member selection mode moves no rows and resizes nothing

Case: `motion-members`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/motion_test.rb:75` — `assert_selector "#channel-members .member-panel__member", minimum: 2, wait: 10` | `rust/reference-tools/users/ledger_browser_members.mjs:516` |
| `test/system/motion_test.rb:79` — `assert_no_selector "#channel-members input[type='checkbox']"` | `rust/reference-tools/users/ledger_browser_members.mjs:517` |
| `test/system/motion_test.rb:84` — `assert_selector "#channel-members input[type='checkbox']"` | `rust/reference-tools/users/ledger_browser_members.mjs:518` |
| `test/system/motion_test.rb:85` — `assert_selector "#channel-members [data-multi-select-target='bar']"` | `rust/reference-tools/users/ledger_browser_members.mjs:518` |
| `test/system/motion_test.rb:86` — `assert_equal lefts_before, member_avatar_lefts, "expected selection mode to shift no rows"` | `rust/reference-tools/users/ledger_browser_members.mjs:520` |
| `test/system/motion_test.rb:87` — `assert_equal content_height_before, member_content_height, "expected the overlaid bar to resize nothing"` | `rust/reference-tools/users/ledger_browser_members.mjs:520` |
| `test/system/motion_test.rb:93` — `assert_selector "#channel-members [data-multi-select-target='messageButton']", text: "Message (2)"` | `rust/reference-tools/users/ledger_browser_members.mjs:523` |
| `test/system/motion_test.rb:94` — `assert_equal lefts_before, member_avatar_lefts, "expected a second selection to shift no rows"` | `rust/reference-tools/users/ledger_browser_members.mjs:524` |
| `test/system/motion_test.rb:95` — `assert_equal content_height_before, member_content_height, "expected a second selection to resize nothing"` | `rust/reference-tools/users/ledger_browser_members.mjs:524` |
| `test/system/motion_test.rb:98` — `assert_no_selector "#channel-members [data-multi-select-target='bar']"` | `rust/reference-tools/users/ledger_browser_members.mjs:526` |
| `test/system/motion_test.rb:99` — `assert_equal lefts_before, member_avatar_lefts, "expected leaving selection mode to shift no rows"` | `rust/reference-tools/users/ledger_browser_members.mjs:526` |
| `test/system/motion_test.rb:100` — `assert_equal content_height_before, member_content_height, "expected hiding the bar to resize nothing"` | `rust/reference-tools/users/ledger_browser_members.mjs:526` |

## P0348: people directory bar shifts no rows when toggling

Case: `motion-directory`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/motion_test.rb:105` — `assert_selector ".people-directory__row", minimum: 2` | `rust/reference-tools/users/ledger_browser_members.mjs:529` |
| `test/system/motion_test.rb:111` — `assert_selector ".multi-select-bar", visible: true` | `rust/reference-tools/users/ledger_browser_members.mjs:530` |
| `test/system/motion_test.rb:112` — `assert_equal tops_before, directory_row_tops, "expected showing the bar to move no rows"` | `rust/reference-tools/users/ledger_browser_members.mjs:531` |
| `test/system/motion_test.rb:115` — `assert_no_selector ".multi-select-bar"` | `rust/reference-tools/users/ledger_browser_members.mjs:533` |
| `test/system/motion_test.rb:116` — `assert_equal tops_before, directory_row_tops, "expected hiding the bar to move no rows"` | `rust/reference-tools/users/ledger_browser_members.mjs:533` |

## P0349: people directory bar stays stuck while scrolling

Case: `motion-sticky`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/motion_test.rb:123` — `assert_selector ".people-directory__row", minimum: 10` | `rust/reference-tools/users/ledger_browser_members.mjs:537` |
| `test/system/motion_test.rb:124` — `assert page.evaluate_script("(() => { const main = document.querySelector('#main-content'); return main.scrollHeight > main.clientHeight + 100; })()"),` | `rust/reference-tools/users/ledger_browser_members.mjs:538` |
| `test/system/motion_test.rb:128` — `assert_selector ".multi-select-bar", visible: true` | `rust/reference-tools/users/ledger_browser_members.mjs:539` |
| `test/system/motion_test.rb:145` — `assert_operator geometry["barBottom"], :<=, geometry["mainBottom"] + 1,` | `rust/reference-tools/users/ledger_browser_members.mjs:543` |

## P0350: room menu measures at full scale when clamping to the viewport edge

Case: `motion-menu`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/motion_test.rb:159` — `assert_selector "#room-menu:not([hidden])"` | `rust/reference-tools/users/ledger_browser_members.mjs:548` |
| `test/system/motion_test.rb:170` — `assert_operator geometry["right"], :<=, geometry["limit"] + 1,` | `rust/reference-tools/users/ledger_browser_members.mjs:551` |
| `test/system/motion_test.rb:174` — `assert_selector "#room-menu[hidden]", visible: :all` | `rust/reference-tools/users/ledger_browser_members.mjs:552` |

## P0351: mobile drawer keeps the room list scroll position across close and reopen

Case: `motion-scroll`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/motion_test.rb:184` — `assert_selector "#sidebar.open"` | `rust/reference-tools/users/ledger_browser_members.mjs:556` |
| `test/system/motion_test.rb:195` — `assert page.evaluate_script("document.querySelector('#{scroller}').scrollHeight > document.querySelector('#{scroller}').clientHeight"),` | `rust/reference-tools/users/ledger_browser_members.mjs:558` |
| `test/system/motion_test.rb:202` — `assert max_scroll >= 400, "expected room for a 400px scroll, got #{max_scroll}px"` | `rust/reference-tools/users/ledger_browser_members.mjs:559` |
| `test/system/motion_test.rb:211` — `assert_equal 400, before` | `rust/reference-tools/users/ledger_browser_members.mjs:561` |
| `test/system/motion_test.rb:212` — `assert_not visible_in_drawer?("#sidebar a[aria-current='page']"),` | `rust/reference-tools/users/ledger_browser_members.mjs:561` |
| `test/system/motion_test.rb:216` — `assert_no_selector "#sidebar.open"` | `rust/reference-tools/users/ledger_browser_members.mjs:562` |
| `test/system/motion_test.rb:224` — `assert_equal before, closed, "expected the room list to keep its scroll position while closed"` | `rust/reference-tools/users/ledger_browser_members.mjs:563` |
| `test/system/motion_test.rb:227` — `assert_selector "#sidebar.open"` | `rust/reference-tools/users/ledger_browser_members.mjs:564` |
| `test/system/motion_test.rb:237` — `assert_equal before, after, "expected the room list to keep its scroll position"` | `rust/reference-tools/users/ledger_browser_members.mjs:567` |
| `test/system/motion_test.rb:238` — `assert visible_in_drawer?(":focus"), "expected focus on a control visible in the drawer"` | `rust/reference-tools/users/ledger_browser_members.mjs:567` |

## P0352: mobile drawer reveals a current room far down the list on first open

Case: `motion-first-reveal`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/motion_test.rb:250` — `assert_selector current_room, visible: :all` | `rust/reference-tools/users/ledger_browser_members.mjs:571` |
| `test/system/motion_test.rb:251` — `assert_equal 0, page.evaluate_script("document.querySelector('#sidebar .sidebar__scroll').scrollTop")` | `rust/reference-tools/users/ledger_browser_members.mjs:572` |
| `test/system/motion_test.rb:252` — `assert_not visible_in_drawer?(current_room), "expected the current room to start out of view"` | `rust/reference-tools/users/ledger_browser_members.mjs:572` |
| `test/system/motion_test.rb:255` — `assert_selector "#sidebar.open"` | `rust/reference-tools/users/ledger_browser_members.mjs:573` |
| `test/system/motion_test.rb:256` — `assert_focused current_room` | `rust/reference-tools/users/ledger_browser_members.mjs:573` |

## P0353: mobile drawer reopens on the current room when it is already in view

Case: `motion-reopen-current`. Status: closed; both final credential-path wrapper runs passed.

| Original assertion | Browser assertion |
| --- | --- |
| `test/system/motion_test.rb:270` — `assert_selector "#sidebar.open"` | `rust/reference-tools/users/ledger_browser_members.mjs:579` |
| `test/system/motion_test.rb:271` — `assert_focused current_room` | `rust/reference-tools/users/ledger_browser_members.mjs:579` |
| `test/system/motion_test.rb:282` — `assert_no_selector "#sidebar.open"` | `rust/reference-tools/users/ledger_browser_members.mjs:581` |
| `test/system/motion_test.rb:286` — `assert_selector "#sidebar.open"` | `rust/reference-tools/users/ledger_browser_members.mjs:582` |
| `test/system/motion_test.rb:287` — `assert_focused current_room` | `rust/reference-tools/users/ledger_browser_members.mjs:582` |
| `test/system/motion_test.rb:288` — `assert_equal before, page.evaluate_script("document.querySelector('#{scroller}').scrollTop"),` | `rust/reference-tools/users/ledger_browser_members.mjs:583` |
