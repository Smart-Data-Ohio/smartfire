# Third-party notices

## gemoji

`rust/web/app/assets/emoji/emoji.json` (the emoji picker's data) was generated
from [gemoji](https://github.com/github/gemoji) by the former Rails app's
`emoji_picker:generate` task and is now committed as data.
`rust/crates/richtext/data/gemoji-4.1.0.json` (the shortcode table for messages
and autocomplete) is derived from gemoji 4.1.0; its license is in
`rust/crates/richtext/data/GEMOJI-LICENSE`.

Copyright (c) 2019 GitHub, Inc.

Released under the MIT License:
https://github.com/github/gemoji/blob/master/LICENSE

## Other vendored code

Other third-party code vendored under `rust/` keeps its license file beside it,
for example `rust/crates/richtext/vendor/html5ever/`,
`rust/crates/rails_compat/vendor/onigmo/`, `rust/crates/assets/vendor/`
(Rails' JavaScript packages and Trix) and
`rust/crates/richtext/data/JSON-PARSER-LICENSE`.
