# Motion: tokens, drawers, menus, toasts

Small transitions throughout the app so drawers slide, menus pop, and
toasts rise instead of snapping. Everything is CSS-first: the only
JavaScript is the huddle toast exit, which waits out its fade before
removing the node.

## Tokens

`app/assets/stylesheets/motion.css` holds the whole vocabulary:

- `--motion-quick: 100ms` — hover and press color changes on buttons
  and rows.
- `--motion-fast: 140ms` — menus, popovers, toasts, banners, badges.
- `--motion-medium: 210ms` — drawer slides and dialog backdrops.
- `--ease-standard: cubic-bezier(0.2, 0, 0, 1)` — every transition above.

New motion must use these tokens, never hardcoded durations. Keyframes
live in `animation.css` (`motion-fade-in`, `motion-rise-in`,
`motion-pop-in`, `motion-panel-in`) and take a token duration with no
fill mode, so a later exit transition on the same element still wins.

## How exits work

- Drawers (mobile sidebar, member panel, thread panel) toggle classes
  over `visibility`. The container transitions `opacity` + `visibility`
  with `allow-discrete`: `visibility` flips immediately on open (so
  focus can move in without waiting, and the surface slides from a real
  before-change style) and only at the end on close, so the surface
  slides out before the drawer unpaints. `inert` still flips
  immediately. Keeping boxes (instead of `display: none`) also preserves
  scroll positions across close and reopen. (Without `allow-discrete`,
  `visibility` would flip at 50% in both directions — too late for
  enter focus, too early for the exit.)
- Popovers and dialogs (room menus, message menu, emoji picker, profile
  card, quick switcher, stage panel) transition `opacity`/`transform`
  plus `overlay`/`display` with `transition-behavior: allow-discrete`,
  so `hidePopover()`, `close()`, and the `hidden` flip wait out the
  fade. `@starting-style` covers the enter.
- The desktop member panel is the exception: it takes a grid column, so
  open/close snaps layout and only the content fades in. The close stays
  instant.

Only `transform` and `opacity` animate (plus the discrete flips above).
Nothing animates width, height, or position on large elements, and
message list inserts never animate, so scroll never jumps.

## Reduced motion and tests

`prefers-reduced-motion: reduce` zeroes the tokens (and the global reset
in `_reset.css` shrinks every transition and animation to 0.01ms), so
everything becomes instant. The layout renders
`data-test-motion="off"` on `<html>` in the test environment for the
same effect, since system tests assert right after acting.
`test/system/motion_test.rb` pins that switch, re-enables motion to
prove the mobile drawer animates in (mid-travel sample plus
`transitionrun` and `getAnimations()`) and lands with focus inside it,
and proves the room list keeps its scroll position while closed and
across reopen.
