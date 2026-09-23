# First-run tour

New members get a five-step tour on their first page load: the sidebar,
the composer, huddles, the Ctrl+K room switcher, and the `?` shortcut
sheet. Each step anchors to its UI target with a highlight ring when the
target exists, and renders the same copy as a centered card when it does
not (a non-room page, an unconfigured huddle, or UI from the not-yet-merged
navigation branch), so the tour works unchanged before and after that
branch lands.

## Behavior

- The `tour` Stimulus controller (`app/javascript/controllers/tour_controller.js`,
  shell in `app/views/layouts/_tour.html.erb`) auto-starts when
  `users.tour_completed_at` is null. Skipping or finishing `PATCH`es
  `users/tours#update`, which stamps the column; the tour never
  auto-starts again afterwards.
- The help menu (`app/views/layouts/_help_menu.html.erb`,
  `help_menu_controller.js`) restarts the tour on demand with a
  `tour:start` window event, without clearing the stamp. It sits in the
  top bar at 40rem and up, and in the sidebar footer on phones, where
  the crowded room header has no room for it. Its Keyboard shortcuts
  item only appears once the navigation branch's `#keyboard-shortcuts`
  dialog exists in the page.
- Bots never see the tour or the help menu.
- Keyboard: the card is a labelled dialog. Escape skips, Left/Right move
  between steps, Tab cycles within the card, and focus returns to the
  invoking element on close.
- Narrow screens get a bottom sheet; `prefers-reduced-motion` disables
  smooth scrolling.
