# Third-party notices

## gemoji

`web/app/assets/emoji/emoji.json` (the emoji picker's data) was generated
from [gemoji](https://github.com/github/gemoji) by the former Rails app's
`emoji_picker:generate` task and is now committed as data.
`crates/richtext/data/gemoji-4.1.0.json` (the shortcode table for messages
and autocomplete) is derived from gemoji 4.1.0; its license is in
`crates/richtext/data/GEMOJI-LICENSE`.

Copyright (c) 2019 GitHub, Inc.

Released under the MIT License:
https://github.com/github/gemoji/blob/master/LICENSE

## Other vendored code

Other vendored third-party code keeps its license file beside it,
for example `crates/richtext/vendor/html5ever/`,
`crates/rails_compat/vendor/onigmo/`, `crates/assets/vendor/`
(Rails' JavaScript packages and Trix), `frontend/tools/oxlint-anti-slop/`
(lint rules, not shipped) and
`crates/richtext/data/JSON-PARSER-LICENSE`.
