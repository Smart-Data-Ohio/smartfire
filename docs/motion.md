# Motion tokens and recipes

`frontend/src/motion/tokens.css` owns the app's durations and easing curves. Components use
those tokens and the recipes under `frontend/src/motion/recipes/`.

Entries and exits have separate durations: micro interactions use 120/80 ms, menus and
popovers 160/120 ms, panels and dialogs 240/180 ms, and route transitions 250/180 ms.
Spring effects use 400 ms. Reduced motion collapses distance, blur and scale into a short fade.
The OS setting applies unless the person chooses a motion override in Settings.

Use the shared presence helpers for enter and exit lifecycles. Verify motion with
`cd frontend && pnpm check` and the relevant mock Playwright specs. Retained auth assets
compile the same motion tokens through `crates/retained_pages/auth_build.rs`.
