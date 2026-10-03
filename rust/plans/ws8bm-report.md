# WS8bm: PR #189 checkpoint and retry review fixes

Branch `rust/ws8bm-messages-http`, based on reviewed `21e4a77d`. Tools/docs only; no Rust product, asset, golden, mask, vendored atom or deadline changed. Rails remains pinned to `d7c7de9264c63015be398001d7a1094e7695a6db` plus the approved drift.

All three findings are fixed with failing-first evidence: post-action rejection attribution, escape retention across invalid retries (both mirror sequences and actual terminal exit), and literal toolbar/picker ARIA selectors. [The audit table, full 159-variant target ledger and raw receipts](ws8bm-review-189-checkpoints.md) record the exact changes, originals, proofs and retained failures. [The earlier labels report](ws8bm-review-189-labels.md) corrects its mistaken literal-selector claim.

Current runs: thirty reviewed cases **30 passed / 0 failed** on both apps; full executable positive set **103 passed / 2 failed**; full discrimination **158 rejected / 1 invalid / 0 escaped**, with six invalid attempts retained. The one invalid final variant failed the already-deferred attachment preview before its filename assertion. The positive failures are that same Rails preview and a Rails permalink-navigation timeout; the exact fallback isolation passes on both apps at its unchanged deadline. All three hidden-allowed probes pass. Helpers **24/24** and Python checks **11/11** pass from the fresh clone.

No declaration is promoted by this review. Frozen historical inventory is **102 passed / 27 deferred / 6 blocked**. The accepted continuation retains **156/156 controllers; 118 passed / 17 deferred / 0 blocked systems** and its concrete remaining-work table. The public list/composer contract in [ws8bm-integration.md](ws8bm-integration.md) stays stable. Duplicate delivery injects browser Turbo markup; it does not verify server-originated redelivery. Release-click remains deferred. Queue-observation races remain inherited.

No new workspace, clippy or release-build claims: this change is confined to reference tools and docs. Own scratch targets are removed after merged verification; logs are retained.
