# First-run tour

New members get a five-step tour on their first page load: the sidebar,
the composer, huddles, the Ctrl+K room switcher, and the `?` shortcut
sheet. Each step anchors to its UI target with a highlight ring when the
target exists, and renders the same copy as a centered card when it does
not, such as a non-room page or an unconfigured huddle.

## Behavior

- The React tour lives in `frontend/src/features/help/`. It starts when `users.tour_completed_at`
  is absent; finishing or skipping records completion through the existing server endpoint.
- The help menu restarts the tour without clearing that stamp.

- Bots never see the tour or the help menu.
- Keyboard: the card is a labelled dialog. Escape skips, Left/Right move
  between steps, Tab cycles within the card, and focus returns to the
  invoking element on close.
- Narrow screens get a bottom sheet; `prefers-reduced-motion` disables
  smooth scrolling.
