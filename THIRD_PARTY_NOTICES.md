# Third-party notices

## gemoji

`crates/static_assets/media/emoji/emoji.json` (the emoji picker's data) was generated
from [gemoji](https://github.com/github/gemoji) by the former Rails app's
`emoji_picker:generate` task and is now committed as data.
`crates/richtext/data/gemoji-4.1.0.json` (the shortcode table for messages
and autocomplete) is derived from gemoji 4.1.0; its license is in
`crates/richtext/data/GEMOJI-LICENSE`.

Copyright (c) 2019 GitHub, Inc.

Released under the MIT License:
https://github.com/github/gemoji/blob/master/LICENSE

## Front-end fonts

`frontend/src/styles/fonts/` holds Latin subsets of two variable fonts, made
by `frontend/tools/fonts/subset.sh` from the upstream releases. Subsetting is a
modification under the SIL Open Font License 1.1; the subsets keep the original
family names, which the license allows because no Reserved Font Name is
declared. Each license is beside the fonts.

- Inter 4.1, Copyright (c) 2016 The Inter Project Authors
  (https://github.com/rsms/inter), SIL Open Font License 1.1:
  `frontend/src/styles/fonts/LICENSE-Inter.txt`.
- JetBrains Mono 2.304, Copyright 2020 The JetBrains Mono Project Authors
  (https://github.com/JetBrains/JetBrainsMono), SIL Open Font License 1.1:
  `frontend/src/styles/fonts/LICENSE-JetBrainsMono.txt`.

## Lucide icons

`frontend/src/ui/icons/icon-data.ts` holds SVG path data generated from
[Lucide](https://lucide.dev) (lucide-static 1.52.0).

Released under the ISC License:
https://github.com/lucide-icons/lucide/blob/main/LICENSE. Copyright for
portions of Lucide is held by Cole Bemis as part of Feather (MIT License,
https://github.com/feathericons/feather/blob/main/LICENSE); all other copyright
for Lucide is held by the Lucide Contributors.

## transitions.dev recipes

The 16 files in `frontend/src/motion/recipes/` are adapted from free recipes
on [transitions.dev](https://transitions.dev) by Jakub Antalik, installed with
`npx transitions-dev add <recipe>`. Each file names its recipe and lists what
changed (mostly: the tunables point at Smartfire's motion tokens, and the
reduced-motion guards follow the app's `data-motion` setting).

They are used and modified under the Transitions.dev License
(https://transitions.dev/terms.html): free recipes may be used and modified in
products, but not republished as a transitions library, template pack or
component kit. Smartfire ships them only as part of its own interface; do not
extract `frontend/src/motion/recipes/` into a standalone package.

## Jakub Antalik's effect libraries

The front end depends on these npm packages, each Copyright (c) 2026 Jakub
Antalik and released under the MIT License (the full text ships in each
package's `LICENSE`):

- `thinking-orbs` 0.3.2 (AgentThinking)
- `border-beam` 1.4.1 (Beam)
- `voice-glow` 0.3.0 (SpeakingRing)
- `bot-avatars` 0.2.2 (AgentAvatar)
- `metal-fx` 2.0.11 (the metal Button variant)

`metal-fx` bundles the `liquidMetal` fragment shader and the sizing vertex
shader from [Paper Shaders](https://github.com/paper-design/shaders),
Copyright (c) Paper Design, Inc., licensed under the Apache License 2.0
(http://www.apache.org/licenses/LICENSE-2.0). Its `NOTICE` file, which
travels with the package, gives the details.

## Bundled notices for the SPA

The React SPA's build copies two notice files from `frontend/public/licenses/`
to `licenses/` in the built dist, beside the SPA's assets.

- LiveKit client 2.22.3 (Apache-2.0), with its dependencies, including
  `@sapphi-red/web-noise-suppressor` 0.4.0 (MIT). The noise suppressor's
  RNNoise WebAssembly is a build of RNNoise (Xiph.Org, BSD-3-Clause) by
  `@shiguredo/rnnoise-wasm` (Apache-2.0). Notices:
  `frontend/public/licenses/livekit-client.NOTICES.txt`.
- Shiki 4.4.3 (MIT), and Highlight.js 11.9.0 (BSD-3-Clause, language
  detection), with their dependencies. Notices:
  `frontend/public/licenses/code-highlighter.NOTICES.txt`.

## Other vendored code

Other vendored third-party code keeps its license file beside it,
for example `crates/richtext/vendor/html5ever/`,
`crates/rails_compat/vendor/onigmo/`, `frontend/tools/oxlint-anti-slop/`
(lint rules, not shipped) and
`crates/richtext/data/JSON-PARSER-LICENSE`.
