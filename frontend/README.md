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

## Coexistence with the classic UI

The SPA replaces the classic pages one screen at a time, and both stay reachable (`SPA_ENABLED`).
`crates/spa/src/screens.rs` maps each classic page to its SPA URL and says whether the SPA has
ported it; `pnpm gen` writes it to `src/gen/screens.json`, which `src/lib/screens.ts` reads.

- A person chooses a UI: "Try the new Smartfire" on the classic profile, "Switch to classic" in
  the menu on the sidebar's user panel (both `POST /app/ui_preference`). `SPA_DEFAULT=next`
  makes the SPA the default for everyone who hasn't chosen.
- For people who use the SPA, a classic HTML GET of a ported screen redirects here; `?classic=1`
  keeps them on the classic page.
- Here, a path the router has no route for but the map names opens on its classic page with a
  full page load (the router's not-found), and clicks on links to ported classic pages open in
  place (`features/shell/classic-links.ts`). Board rooms open on the classic board until boards
  are ported.

Porting a screen: add its route, flip `ported` in `screens.rs` in the same PR, and run `pnpm gen`
(`src/router.test.ts` fails until the two agree).

## URL cutover contract

[`crates/spa/compat/urls.json`](../crates/spa/compat/urls.json) pins incoming navigation URLs,
their current responses, and their destinations after the root cutover. It includes old
bookmarks and notification links, `/app/` deep links, and the auth, public, PWA, file, API and
socket URLs that root routing must reserve. This is an acceptance contract; runtime routing
still comes from the route table and screen map.

Later default, root-dispatch and prefix changes should update the affected expectations in
this JSON alongside the implementation. Keep the old incoming paths as cases, preserve their
query context, and keep explicit classic choices and `classic=1` until classic retirement.
The Rust HTTP test reads every entry from the file, so changing an expected response requires
no test-code edit. A missing screen's eventual destination records required future work, not
a claim that the screen exists today.

Each entry records `owner_today`, `after_cutover`, bookmark and notification dependencies,
and request `cases`. Update `cases[].expect` for a later step's status, redirect or content
marker; `expect_stub` and `expect_built` distinguish a Cargo-only shell from the built SPA.
Keep `after_cutover` as the final destination and retain cases for each older incoming URL.
`known_bug.fix_before` identifies blockers for `default-next` or `classic-retirement`;
`none` records an unrelated defect. Fixes should update the observed expectations alongside
the implementation, rather than preserving a bug through the cutover.

Authenticated integration cases use private fixture profiles and optional `before`/`after`
HTTP steps to establish OAuth state or verify a successful mutation. Machine requests specify
`csrf: false` so browser-session bootstrap cannot hide an accidental CSRF dependency. CSRF
tokens come from the session cookie, with the retained sign-in page as a fallback. Successful
file responses can pin their bytes with `body_sha256`; redirects compare both origin and path.
The `follow_location` expectation fetches the returned same-origin URL and checks its response,
including signatures and file bytes. `expect.json` parses the body, matches object fields
recursively, and requires each declared array element to appear (array order is immaterial).
Use nonempty objects or arrays with representative identities and values.

The `browser_cases` array pins fragments and query strings through actual browser navigation,
so later routing changes update these expectations in the same JSON file.

Restore the frozen seeds before running the contract tests from the repository root:

```sh
python3 parity/bin/frozen-seeds restore
cargo test -j 4 -p campfire url_contract_tests -- --nocapture
```

CI runs the mock suite in three Frontend e2e shards. The first shard also runs
production-preview tests, including the PWA checks. Ordinary Rust tests retain
the URL contract, manifests, worker selection and response-header checks.
The separate real-server SPA and PWA browser suites have been removed.
Deploy verifies public auth pages and the enabled SPA's built offline shell and
asset responses after health succeeds.

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
