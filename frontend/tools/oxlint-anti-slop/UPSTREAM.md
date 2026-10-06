# Anti-slop provenance

Vendored from [`dmmulroy/anti-slop`](https://github.com/dmmulroy/anti-slop) at commit
`c44ef22ca116d0ba62a3ff663a0bd13a3f3fa40b` (2026-09-10), copied with the upstream
`install-anti-slop` skill's `install.mjs` (its `assets/anti-slop` bundle, which is `src/` without
the rule tests). `LICENSE` is upstream's MIT licence. `vendor/eslint-stylistic/` carries its own
licence and provenance.

This is a later commit than the copy in Smart-Ops (`446268e5`): the optional Effect rules
(`effect/index.ts`) were added upstream after that one.

The rules are repository-owned code now, as upstream intends. To update them, run the current
upstream installer into a scratch directory, review the diff against this copy, and update the
commit above. `.oxlintrc.json` enables every generic rule and every Effect rule.

`tsconfig.json` is ours: upstream's compiler options, so `pnpm typecheck` checks the plugin apart
from the app's stricter settings.
