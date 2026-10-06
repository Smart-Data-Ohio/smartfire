# Smartfire front end

The React 19 single-page app that replaces the Hotwire UI (`web/`) screen by screen. The Rust app
will serve the production build under `/app/`; production runs no Node.

Stack: Vite 8 (Rolldown), React 19 with the React Compiler (Babel preset), typescript@7, Effect 4
in the data layer, Biome, the vendored anti-slop Oxlint rules, Vitest and Playwright. pnpm, with
the versions pinned in `package.json` and `pnpm-lock.yaml`.

## Commands

Run from `frontend/`, with Node 24 or later and pnpm (pnpm switches itself to the version in
`package.json`'s `packageManager` field):

| Command | What |
| --- | --- |
| `pnpm install` | Install the locked dependencies |
| `pnpm dev` | Vite dev server on http://localhost:5173/app/; it proxies `/api`, `/cable`, `/rails` and `/session` to `cargo run` on :3000 |
| `pnpm build` | Production build into `dist/` (asset URLs under `/app/`) |
| `pnpm preview` | Serve `dist/` |
| `pnpm gen` | Regenerate `src/gen/` from the Rust API types (`crates/api_types`, through `cargo test -p campfire_api_types export_bindings`; needs the Rust toolchain). Commit the result: CI fails if it differs |
| `pnpm lint` | Biome lint and format check (`pnpm format` applies fixes) |
| `pnpm lint:anti-slop` | The anti-slop Oxlint rules |
| `pnpm typecheck` | `tsc --noEmit` for the app, then for the vendored plugin |
| `pnpm test` | Vitest unit tests (`src/**/*.test.ts(x)`) |
| `pnpm test:e2e` | Playwright specs in `e2e/` against `vite preview` (first run: `pnpm exec playwright install chromium`); set `SHOTS_DIR=/some/dir` to also write kitchen-sink screenshots there (light, dark, compact; not committed) |
| `pnpm motion:scan` | Score the motion locally with `transitions-agent scan` (static analysis, no upload). Never run its `fix` or `--pr` modes: they upload code |
| `pnpm check` | lint, anti-slop, typecheck, test and build: what the `Frontend` CI check runs |

## Layout

`src/main.tsx` mounts the app. `src/api/` is the HTTP client and `src/sync/` the live-update
engine and the shared Effect runtime; `src/store/` holds client state, `src/routes/` the routes,
`src/features/` one folder per product area, `src/ui/` the design system, and `src/motion/`,
`src/styles/` and `src/lib/` the shared pieces. `e2e/` holds the Playwright specs.

## API types

`src/gen/` holds the TypeScript types ts-rs generates from the Rust DTOs in `crates/api_types`
(`pnpm gen`); never edit it by hand. The Effect Schemas that decode the API's JSON are written by
hand in `src/api/schema/` and `src/api/errors.ts`, and each exports a
`...Pin = Assert<Pinned<typeof Schema, GeneratedType>>` type: `tsc` fails until a schema's wire
(encoded) side matches its generated type both ways. The decoded side adds branded ids
(`UserId`, `RoomId`, ...) and `DateTime.Utc` timestamps. After changing a Rust DTO, run
`pnpm gen`, then update the schema until `pnpm typecheck` passes.

## Design system

`/app/_kitchen-sink` (the `pnpm dev` server, or a build) shows every component in every variant
and state, a miniature of the app, and switches for theme, density, motion and a side-by-side
light/dark view. Check changes to `src/ui/`, `src/motion/` or `src/styles/` there.

- `src/styles/`: OKLCH design tokens (`tokens.css`; every colour is a `light-dark()` pair, themed
  by `:root[data-theme]` or the OS), the type scale, the base layer and the self-hosted Inter and
  JetBrains Mono subsets (`tools/fonts/subset.sh` rebuilds them). `tokens.test.ts` holds every
  text token to 4.5:1 on the surfaces it sits on, in both themes. Components use only the
  semantic tokens; violet (`--agent`) is reserved for agents.
- `src/motion/`: duration, easing, distance and scale tokens (reduced motion folds into the
  tokens, and `<html data-motion>` overrides the OS), `usePresence` for exit animations, and
  `recipes/`, 16 adapted transitions.dev recipes (license in `THIRD_PARTY_NOTICES.md`: they ship
  inside the app, never as a standalone kit).
- `src/ui/`: the components. Overlays use the platform: `<dialog>`, the Popover API and CSS
  anchor positioning (with a JS fallback in `src/lib/anchor.ts`), so there is no headless UI
  dependency. Jakub Antalik's canvas effects (`thinking-orbs`, `border-beam`, `voice-glow`,
  `bot-avatars`, `metal-fx`) are wrapped as AgentThinking, Beam, SpeakingRing, AgentAvatar and
  the metal Button, each lazy-loaded with a static fallback for reduced motion.
- `src/lib/appearance.ts`: the per-device theme, density and motion settings.

Stylesheets share one cascade-layer order, `tokens, base, motion, recipes, ui, app`, so a
component's styles beat a recipe's without specificity fights.

## The Effect boundary

Effect is used only in `src/api/` and `src/sync/`. Everything else in `src/` (components, routes,
the store, the design system) is plain TypeScript and React, and calls the plain async functions
in `src/sync/runtime.ts` (`actions`), which run Effect programs on the app's one
`ManagedRuntime`. Biome's `noRestrictedImports` (the override in `biome.json`) rejects `effect`,
`effect/*` and `@effect/*` imports anywhere else in `src/`. Test Effect code with
`@effect/vitest`.

## Anti-slop

`tools/oxlint-anti-slop/` is a vendored copy of
[dmmulroy/anti-slop](https://github.com/dmmulroy/anti-slop) (MIT; commit and update steps in
its `UPSTREAM.md`). Oxlint runs only these rules, configured in `.oxlintrc.json`: every generic
rule plus the optional Effect rules, all at `error`. Biome does the general linting. The rules
are repository-owned code: change them deliberately, and fix findings rather than disabling a
rule or casting around it.
