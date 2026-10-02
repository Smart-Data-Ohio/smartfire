# Pinned Selenium visibility atom

`isDisplayed.js` is an unmodified copy from the Rails reference image at
`d7c7de9264c63015be398001d7a1094e7695a6db`:

`selenium-webdriver-4.35.0/lib/selenium/webdriver/atoms/isDisplayed.js`

SHA256: `ae26018c01cd27448b250f8e55a094cbfcd2e2cbbe171c78aaa906e1b5c3ed7c`.
The accompanying Apache 2.0 LICENSE and NOTICE come from the same installed gem.

`behavior-visibility.mjs` calls the atom with `ignoreOpacity=false`, just as
Selenium's Ruby atom adapter does by default. It checks effective opacity
through ancestors as well as Selenium's display, visibility, geometry,
overflow, details and shadow-tree rules. It runs only in reference checks;
it is not served by either application or included in Rust build inputs.

The helper counts visible locator matches and polls within the existing
assertion deadline. Explicit `attached`/`detached` checks retain DOM-presence
semantics. JavaScript geometry probes copied from the original Rails tests
retain those tests' own `getClientRects()` measurements.

`visibleText.js` contains the unmodified visible-text routines from
`javascript/atoms/dom.js` at Selenium's `selenium-4.35.0` tag, matching the
installed reference version. `behavior-text.mjs` supplies their Closure
array/string/DOM dependencies and delegates `isShown` to the pinned atom above
with opacity respected. The routines preserve Selenium's whitespace, text
transform, table-cell and composed shadow-DOM behavior.
Source: https://github.com/SeleniumHQ/selenium/blob/selenium-4.35.0/javascript/atoms/dom.js
