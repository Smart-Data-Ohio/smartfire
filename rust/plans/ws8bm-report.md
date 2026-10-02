# WS8bm PR #189 review corrections — PR-ready partial slice

Worker: GPT-6.1 Sol. Branch: `rust/ws8bm-messages-http`.
Reference: Rails `d7c7de9264c63015be398001d7a1094e7695a6db`, plus the approved drift in wave4/_common.md. Behaviour and response parity only; no pixel checks.

The three requested review defects are corrected. Current counts are **102 passed / 27 remaining / 6 owner-blocked** out of 135 mapped system declarations, and **146/156 controller declarations**, with all ten remaining controller declarations assigned to WS12. These are scoped mapped Playwright checks, not execution of the original Ruby system files or full system sign-off. This slice adds no declaration credit and returns the release-click declaration to deferred after two fresh reference failures. The three requested review corrections form a coherent review-ready slice; full system sign-off and a green workspace gate remain outstanding.

**Duplicate-delivery limitation:** the check injects Turbo markup in the browser, using the actual mounted row and render queue. It verifies the browser render guard and object identity. It **does not verify server-originated redelivery**; the original Rails case calls `message.broadcast_create`. This limitation is now recorded beside the declaration in the inventory as well as here. End-to-end server redelivery remains unverified by this mapped check.

## Changes and failing-first evidence

- Merge `8abc3119bba23e823eba4e3dfb87271b7f20c0ba` brings in `origin/main` at `3ab3a4db58e094cea0f41b90e63f54d59ce9309e`, including #184. The merge was clean and retains both sides; no stash. Main's literal freshness digest and test-only vector include remain intact. Locked Cargo metadata passed; all fifteen tracked manifests parsed without duplicate dependency keys.
- Fix `4d88ea4f38df5d2c04f824cbd714ce4b22cdca95` restores highlighted `const` in search and initial edit code, and highlighted `def` in edited Python. Each keyword uses one original 20-second assertion, not sequential readiness waits. Three additional served mutants remove only the relevant keyword token classes while preserving the source and other tokens.
- The action check requires exactly eight **visible** quick reactions within the original two-second default. The additional mutant hides only Clapping while retaining eight DOM elements.
- All thirty checks were audited against the four pinned originals; [ws8bm-assertion-timeouts.md](ws8bm-assertion-timeouts.md) records each declaration's deadlines. The original Capybara default is two seconds (verified in the pinned installed gem), reaction delivery is ten seconds, highlighting remains twenty seconds and long press remains 700 ms. Positive and mutant runs use identical assertion budgets. The delayed-write mutant holds the actual boost POST for eleven seconds before forwarding, so its write and Cable delivery cannot occur inside the ten-second budget. Author active-state assertions run before the extra peer check. Ruby immediate focus comparisons remain immediate. Native timing tests, concurrency and rustc throttling are unchanged.
- All thirty original source-anchored mutants remain. The five additional variants get independent seed/database/server copies, including when a rejected highlight check has already saved a message. Mutation results require valid case startup, an applied mutation, no actual network failure, and rejection on **both** applications. Diagnostic escape probes cannot earn parity credit.

Before changing the assertions, ran each new variant over the unchanged received `6427c785` assertion modules against pinned Rails and the merged Rust app. All five escaped on both applications:

```text
WS8bm review escape: code_highlighting: search results highlight code on initial load and after returning to the channel: missing-const: Rails ACCEPTED
WS8bm review escape: code_highlighting: search results highlight code on initial load and after returning to the channel: missing-const: Rust ACCEPTED
WS8bm review escape check: 1 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
WS8bm review escape: code_highlighting: editing a code block replaces its language colors and copied source: missing-const: Rails ACCEPTED
WS8bm review escape: code_highlighting: editing a code block replaces its language colors and copied source: missing-const: Rust ACCEPTED
WS8bm review escape check: 1 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
WS8bm review escape: code_highlighting: editing a code block replaces its language colors and copied source: missing-def: Rails ACCEPTED
WS8bm review escape: code_highlighting: editing a code block replaces its language colors and copied source: missing-def: Rust ACCEPTED
WS8bm review escape check: 1 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
WS8bm review escape: message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press: hidden-clapping: Rails ACCEPTED
WS8bm review escape: message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press: hidden-clapping: Rust ACCEPTED
WS8bm review escape check: 1 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
WS8bm review escape: message_toolbar: quick-react creates a boost from the toolbar: delayed-boost-write: Rails ACCEPTED
WS8bm review escape: message_toolbar: quick-react creates a boost from the toolbar: delayed-boost-write: Rust ACCEPTED
WS8bm review escape check: 1 served mutants accepted on Rails and Rust; 0 failed probes; no parity credit
```

The following diagnostic commands were rerun before changing the assertions (no parity credit):

```sh
python3 rust/reference-tools/messaging/behavior-check.py code_highlighting --case 'search results highlight code on initial load and after returning to the channel' --mutant missing-const
python3 rust/reference-tools/messaging/behavior-check.py code_highlighting --case 'editing a code block replaces its language colors and copied source' --mutant missing-const
python3 rust/reference-tools/messaging/behavior-check.py code_highlighting --case 'editing a code block replaces its language colors and copied source' --mutant missing-def
python3 rust/reference-tools/messaging/behavior-check.py message_interactions --case 'opens message actions from context menu and keyboard, and cancels a moving long press' --mutant hidden-clapping
python3 rust/reference-tools/messaging/behavior-check.py message_toolbar --case 'quick-react creates a boost from the toolbar' --mutant delayed-boost-write
```

The subsequent fresh-clone discrimination run below is the corrected regression evidence.

Earlier slices, command results and attribution are retained in the [received report at 6427c785](https://github.com/Smart-Data-Ohio/smartfire/blob/6427c78560b9d9095e711c9d984b223319f445b8/rust/plans/ws8bm-report.md). Those historical commands and counts are not represented as rerun results here.

## Stable integration contract

The shell seam in [ws8bm-integration.md](ws8bm-integration.md) is unchanged: `Presenter::messages(&records)`, `messages::Index { ctx, messages }`, and `Presenter::room_message_list(&records, divider.message_id, divider.count)`. The shell passes verified request origin/viewer/assets/signer/CSRF context, selected records and divider facts. The composer receives room kind/name/ID, ordered commands, Drive flow, thread scope, the real scheduled-message child and request-owned PendingTemplate. Mount list/template boundaries verbatim. Real WS15g GitHub rendering and merged WS14e/M2 children remain in use. There is no new cross-owner production touch.


## Current verification

The worktree's initial corrected paired run passed all thirty cases with saved-row checks. The full committed fresh-source clone then ran all requested commands, including every workspace crate despite the application failure, strict clippy with warnings denied, all thirty paired cases, and all thirty-five served mutants on both apps.

Fresh native totals: **3,881 passed / 1 failed / 12 inherited ignores**, from 58 raw summaries. Strict clippy passed. All **35 mutants were rejected on both applications, zero invalid or escaped**. The fresh paired batch returned **29 passed / 1 reference failure**. The fresh wrapper therefore exits nonzero; the overall workspace gate is **not green**.

The native failure is inherited from the merged WS16/main source, unchanged by this worker (the client and its test are identical to origin/main at 3ab3a4db). It was introduced in `906efac51ac6b50762b20bd84407bff5f599b5ed`. The precise failure is:

```text
---- integrations::slack::client::tests::slack_client_pacing_is_shared_per_tier_and_separate_between_tiers stdout ----

thread 'integrations::slack::client::tests::slack_client_pacing_is_shared_per_tier_and_separate_between_tiers' (30964) panicked at crates/campfire/src/integrations/slack/client/tests.rs:393:5:
assertion failed: first.elapsed() >= Tier::Two.interval()
```

No attempt was made to repair this timing case or `the_last_member_out_destroys_the_group`, reduce concurrency, add ignores, or widen thresholds. The pacing failure stays with the timing/WS16 owner.

The fresh reference-only release-click failure was `DIV !== menu` at behavior-actions.mjs:89, before Rust's paired execution of that declaration. The original requires the press point to hit the menu, so its assertion was retained. It is reported as an observed reference failure, not a Rust parity defect or a credited successful run. A single unchanged-budget recheck is recorded below; it does not erase the first failure.

Reran from the worktree root:

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_toolbar message_actions_mobile message_interactions code_highlighting --keep-going
python3 rust/reference-tools/messaging/fresh-check.py --behavior-files message_toolbar message_actions_mobile message_interactions code_highlighting
mise exec rust@1.98.1 -- cargo metadata --locked --manifest-path rust/Cargo.toml --format-version 1 > /dev/null
```

The metadata command was followed by `tomllib` parsing every tracked Cargo manifest, so duplicate dependency keys are rejected. Its raw result:

```text
WS8bm merge metadata: locked Cargo metadata passed; 15 manifests parsed without duplicate dependency keys
```

Raw initial corrected paired summary:

```text
WS8bm behaviour check: 30 named cases passed on Rails and Rust; 0 failed; no pixel checks
```

Raw fresh summaries (all 58 native lines, each paired result and both-app mutant results):

```text
WS8bm fresh checkout: 4d88ea4f38df5d2c04f824cbd714ce4b22cdca95; no pre-existing scratch or Cargo target; /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-fresh-orbvnk35
WS8bm fresh concurrency: eight test threads; two build jobs; native timing unchanged; audited browser deadlines match Rails
WS8bm pinned processing: campfire-toolchain; shared machine rustc flock slots
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 7m 37s
test result: FAILED. 1980 passed; 1 failed; 3 ignored; 0 measured; 0 filtered out; finished in 1595.70s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.74s
test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 41.87s
test result: ok. 1 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 9.82s
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.01s
test result: ok. 1194 passed; 0 failed; 4 ignored; 0 measured; 0 filtered out; finished in 199.51s
test result: ok. 52 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.86s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.05s
test result: ok. 119 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.39s
test result: ok. 15 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 4.02s
test result: ok. 32 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.26s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 54 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 5.40s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.29s
test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.57s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 39.66s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.26s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.11s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.44s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 16.28s
test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.24s
test result: ok. 44 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.90s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.15s
test result: ok. 79 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 16.50s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 2 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
workspace: exit 101; inspect /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-fresh-orbvnk35/.scratch/workspace.log
test integrations::slack::client::tests::slack_client_pacing_is_shared_per_tier_and_separate_between_tiers ... FAILED
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 19s
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m 23s
WS8bm behaviour: message_toolbar: the toolbar stays hidden until hover or focus and labels every action: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: reply and thread buttons drive the composer and the thread panel: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: the more button opens the shared menu for its message: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: the picker shows category tabs and switches between them: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: the picker loads its emoji data only on first open: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: picker tabs move with arrow keys and switch the grid: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: quick-react creates a boost from the toolbar: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: keyboard users reach the toolbar from a focused message: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: the emoji picker searches and reacts: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: the picker remembers recent reactions: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: the picker Custom tab reacts with a workspace icon: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: the picker reacts with a brand icon shortcode: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_toolbar: picker arrows move through options, Enter selects, and Escape returns focus: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour source: test/system/message_toolbar_test.rb SHA256 d0e63bfb2c31db13f5786afd5ff28728f7dffead32468143fe3cb513598fdc12
WS8bm behaviour: message_actions_mobile: message action menu is a bottom sheet with touch-sized targets on phones: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_actions_mobile: message action menu stays a floating popover on desktop: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour source: test/system/message_actions_mobile_test.rb SHA256 4d930fbf29081825c2696f1be81ec062b0e6968031b93fef02d47b08818e856d
WS8bm behaviour: message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_interactions: shows the message action menu as a bottom sheet on phones: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_interactions: a duplicate delivery does not replace the message while its actions are open: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_interactions: keeps newer typing through an asynchronous edit and leaves failures in edit mode: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_interactions: edits through the normal composer and restores the saved draft on cancel and success: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_interactions: replies with notify off and renders a tombstone when the target is deleted: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_interactions: copies message text and link and forwards to a server-provided thread destination: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_interactions: forwarding twice in a row submits only once: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: message_interactions: groups emoji reactions, updates the live count, and highlights the current user: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour source: test/system/message_interactions_test.rb SHA256 4c867b252b0697ea694a9a3637b4242195638e261b62ccfe6e90ff21ea20d8db
WS8bm behaviour: code_highlighting: language fences highlight common code without changing its text: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: code_highlighting: unlabelled code is detected while text unknown languages and inline code stay literal: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: code_highlighting: search results highlight code on initial load and after returning to the channel: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: code_highlighting: code and copying remain available when the highlighter cannot load: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour: code_highlighting: editing a code block replaces its language colors and copied source: Rails PASS; Rust PASS; persisted rows PASS
WS8bm behaviour source: test/system/code_highlighting_test.rb SHA256 bd8e2522a1fab8e0054b401d199acc3a942f4e4be866b7b1c011de650bed2014
WS8bm behaviour check: 29 named cases passed on Rails and Rust; 1 failed; no pixel checks
behavior: exit 1; inspect /home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-fresh-orbvnk35/.scratch/behavior.log
seed: building default
seed: default -> parity/.seed/default (6.1M)
seed: building first_run
seed: first_run -> parity/.seed/first_run (1.5M)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.23s
WS8bm discrimination: message_toolbar: the toolbar stays hidden until hover or focus and labels every action: default: Rails served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_toolbar: the toolbar stays hidden until hover or focus and labels every action: default: Rust served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_toolbar: reply and thread buttons drive the composer and the thread panel: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: reply and thread buttons drive the composer and the thread panel: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the more button opens the shared menu for its message: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the more button opens the shared menu for its message: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the picker shows category tabs and switches between them: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the picker shows category tabs and switches between them: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the picker loads its emoji data only on first open: default: Rails served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_toolbar: the picker loads its emoji data only on first open: default: Rust served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_toolbar: picker tabs move with arrow keys and switch the grid: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: picker tabs move with arrow keys and switch the grid: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: quick-react creates a boost from the toolbar: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: quick-react creates a boost from the toolbar: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: keyboard users reach the toolbar from a focused message: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: keyboard users reach the toolbar from a focused message: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the emoji picker searches and reacts: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the emoji picker searches and reacts: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the picker remembers recent reactions: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the picker remembers recent reactions: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the picker Custom tab reacts with a workspace icon: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the picker Custom tab reacts with a workspace icon: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the picker reacts with a brand icon shortcode: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: the picker reacts with a brand icon shortcode: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: picker arrows move through options, Enter selects, and Escape returns focus: default: Rails served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_toolbar: picker arrows move through options, Enter selects, and Escape returns focus: default: Rust served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_toolbar: quick-react creates a boost from the toolbar: delayed-boost-write: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_toolbar: quick-react creates a boost from the toolbar: delayed-boost-write: Rust served mutant REJECTED (TimeoutError)
WS8bm behaviour source: test/system/message_toolbar_test.rb SHA256 d0e63bfb2c31db13f5786afd5ff28728f7dffead32468143fe3cb513598fdc12
WS8bm discrimination: message_actions_mobile: message action menu is a bottom sheet with touch-sized targets on phones: default: Rails served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_actions_mobile: message action menu is a bottom sheet with touch-sized targets on phones: default: Rust served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_actions_mobile: message action menu stays a floating popover on desktop: default: Rails served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_actions_mobile: message action menu stays a floating popover on desktop: default: Rust served mutant REJECTED (ERR_ASSERTION)
WS8bm behaviour source: test/system/message_actions_mobile_test.rb SHA256 4d930fbf29081825c2696f1be81ec062b0e6968031b93fef02d47b08818e856d
WS8bm discrimination: message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: a release click landing on the just-opened menu does not activate it: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: a release click landing on the just-opened menu does not activate it: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: shows the message action menu as a bottom sheet on phones: default: Rails served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_interactions: shows the message action menu as a bottom sheet on phones: default: Rust served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_interactions: a duplicate delivery does not replace the message while its actions are open: default: Rails served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_interactions: a duplicate delivery does not replace the message while its actions are open: default: Rust served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_interactions: keeps newer typing through an asynchronous edit and leaves failures in edit mode: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: keeps newer typing through an asynchronous edit and leaves failures in edit mode: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: edits through the normal composer and restores the saved draft on cancel and success: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: edits through the normal composer and restores the saved draft on cancel and success: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: replies with notify off and renders a tombstone when the target is deleted: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: replies with notify off and renders a tombstone when the target is deleted: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: copies message text and link and forwards to a server-provided thread destination: default: Rails served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_interactions: copies message text and link and forwards to a server-provided thread destination: default: Rust served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_interactions: forwarding twice in a row submits only once: default: Rails served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_interactions: forwarding twice in a row submits only once: default: Rust served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: message_interactions: groups emoji reactions, updates the live count, and highlights the current user: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: groups emoji reactions, updates the live count, and highlights the current user: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press: hidden-clapping: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: message_interactions: opens message actions from context menu and keyboard, and cancels a moving long press: hidden-clapping: Rust served mutant REJECTED (TimeoutError)
WS8bm behaviour source: test/system/message_interactions_test.rb SHA256 4c867b252b0697ea694a9a3637b4242195638e261b62ccfe6e90ff21ea20d8db
WS8bm discrimination: code_highlighting: language fences highlight common code without changing its text: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: code_highlighting: language fences highlight common code without changing its text: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: code_highlighting: unlabelled code is detected while text unknown languages and inline code stay literal: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: code_highlighting: unlabelled code is detected while text unknown languages and inline code stay literal: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: code_highlighting: search results highlight code on initial load and after returning to the channel: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: code_highlighting: search results highlight code on initial load and after returning to the channel: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: code_highlighting: code and copying remain available when the highlighter cannot load: default: Rails served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: code_highlighting: code and copying remain available when the highlighter cannot load: default: Rust served mutant REJECTED (ERR_ASSERTION)
WS8bm discrimination: code_highlighting: editing a code block replaces its language colors and copied source: default: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: code_highlighting: editing a code block replaces its language colors and copied source: default: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: code_highlighting: search results highlight code on initial load and after returning to the channel: missing-const: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: code_highlighting: search results highlight code on initial load and after returning to the channel: missing-const: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: code_highlighting: editing a code block replaces its language colors and copied source: missing-const: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: code_highlighting: editing a code block replaces its language colors and copied source: missing-const: Rust served mutant REJECTED (TimeoutError)
WS8bm discrimination: code_highlighting: editing a code block replaces its language colors and copied source: missing-def: Rails served mutant REJECTED (TimeoutError)
WS8bm discrimination: code_highlighting: editing a code block replaces its language colors and copied source: missing-def: Rust served mutant REJECTED (TimeoutError)
WS8bm behaviour source: test/system/code_highlighting_test.rb SHA256 bd8e2522a1fab8e0054b401d199acc3a942f4e4be866b7b1c011de650bed2014
WS8bm discrimination check: 35 served mutants rejected on Rails and Rust across 30 named checks; 0 invalid or escaped
WS8bm fresh target removed
AssertionError: WS8bm fresh-check failures: workspace, behavior; all available checks were run
WS8bm delayed-write probe: Rails: 10013 ms observed; actual write completed: false
WS8bm delayed-write probe: Rust: 10007 ms observed; actual write completed: false
```

The unchanged-budget recheck ran from that same committed source clone after verifying its revision and the absence of a Cargo target. It rebuilt its own seeds/binary and used CI=1, eight test threads, two build jobs and its own Cargo target:

```sh
python3 rust/reference-tools/messaging/behavior-check.py message_interactions --case 'a release click landing on the just-opened menu does not activate it'
```

It reproduced the same pinned Rails failure. The case is **deferred**, reducing scoped mapped passes from 103 to 102 and increasing remaining declarations from 26 to 27. Its mutation rejection does not close this unresolved positive-reference failure.

```text
WS8bm failed application: http://127.0.0.1:52020 a release click landing on the just-opened menu does not activate it
WS8bm browser flow FAILED: message_interactions: a release click landing on the just-opened menu does not activate it: AssertionError [ERR_ASSERTION]: Expected values to be strictly equal:

'DIV' !== 'menu'

    at interactions (file:///home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-fresh-orbvnk35/rust/reference-tools/messaging/behavior-actions.mjs:89:12)
    at async acceptance (file:///home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-fresh-orbvnk35/rust/reference-tools/messaging/behavior.mjs:133:7)
    at async file:///home/riels/Projects/SD-Labs/Campfire/.claude/worktrees/rust-ws8bm/.scratch/ws8bm-fresh-orbvnk35/rust/reference-tools/messaging/behavior.mjs:503:9
WS8bm unchanged-budget reference recheck: exit 1
WS8bm recheck target removed
```

All current report commands were executed in this session; historical command lists are linked, not represented as current passes. Node syntax checks for all changed modules, Python compilation, and git diff --check passed. The source hashes and 135 exact pinned declaration names were verified again. This review slice adds no non-test Rust include; its changes are confined to reference tools and plans. The stable presenter/list/composer seam and the WS12 activity/work/board ownership boundary are unchanged.

Final verification and cleanup:

```text
WS8bm pinned system inventory: 102 passed / 27 remaining / 6 owner-blocked; 135 exact names and source hashes verified
WS8bm cleanup: zero scratch Cargo targets remain; assigned 52020/52021/52022 listeners and parity-owner containers absent
```

## Exact remaining work

### Remaining WS8bm behaviour (27)

`test/system/message_interactions_test.rb`

- a release click landing on the just-opened menu does not activate it — repeated pinned Rails failure; keep its original assertion and deadlines; positive behaviour integration remains unresolved

`test/system/code_highlighting_test.rb`

- thread code stays readable in both themes and scrolls within a narrow screen

`test/system/threads_test.rb`

- keeps the thread drawer usable on a phone and preserves the channel
- marks a joined thread read only while the conversation is visible
- opens a shared thread message link around an older post
- keeps an anchored older thread unread when a new reply arrives
- discusses a pull request from its card

`test/system/workspace_markdown_test.rb`

- Markdown replies and file attachments remain usable
- workspace follows the system theme and mobile navigation remains reachable

`test/system/composer_attach_menu_test.rb`

- From Google Drive starts the legacy picker flow
- From Google Drive starts the enhanced share flow when sharing is configured

`test/system/drive_attachments_test.rb`

- attach Drive files from the picker, send textless, and remove through edit
- edit a room message in the composer and remove one of two attachments
- attach a Drive file from the thread composer

`test/system/motion_test.rb`

- motion is off by default in the test environment
- mobile drawer animates in, lands in place, and returns focus with motion on
- member selection mode moves no rows and resizes nothing
- people directory bar shifts no rows when toggling
- people directory bar stays stuck while scrolling
- room menu measures at full scale when clamping to the viewport edge
- mobile drawer keeps the room list scroll position across close and reopen
- mobile drawer reveals a current room far down the list on first open
- mobile drawer reopens on the current room when it is already in view

`test/system/mobile_layout_test.rb`

- the profile page fits phone widths without scrolling sideways
- headers outside the workspace shell stay opaque over scrolled content
- headers outside the workspace shell never cover the page or its scrollbar
- pages outside the workspace shell show no drawer toggle that opens nothing

### Owner-blocked system declarations (6)

`test/system/threads_test.rb`

- tracks work, assigns an owner, completes and reopens it without losing the conversation
- shows work-thread guidance in the new-thread form and on the work page
- keeps the new-thread guidance usable on a phone
- shows work assignment activity to the owner and opens the exact thread

`test/system/message_list_a11y_test.rb`

- text fields stay at 16px on touch devices without changing the desktop look

`test/system/mobile_layout_test.rb`

- every drawer destination has one toggle that opens the drawer on itself

### Owner-blocked controller declarations (10)

All are in `test/controllers/channel_threads_controller_test.rb`, awaiting WS12 board/work writes and agent work services.

- converts a thread to work, assigns an eligible owner, and keeps an audit trail
- work owner must be an eligible parent-room member and a revoked owner stays visible as unavailable
- assigned owner can change work status but cannot reassign it
- only a thread manager can remove work tracking
- the work model also protects conversion when the owner field is omitted
- work status updates from separate stale instances produce one event per real change
- a manager can assign an eligible agent and the agent is notified
- the owner picker lists eligible agents with profiles and excludes ineligible ones
- a member who cannot manage the thread cannot assign an agent
- ordinary thread fields remain separate from work tracking
