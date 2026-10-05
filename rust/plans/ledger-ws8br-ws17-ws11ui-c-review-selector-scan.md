# PR #248 selector and visibility scan

This is the review follow-up to `dae59026`. All 120 declarations were checked against the runtime Rails pin `78b9b1546bdab4c6c1c9b8ddb94512f661289112`. The two assertion tables preserve the exact Rails assertion → Rust assertion file:line mapping. Historical `d7c7de92` mappings remain identified in JSON history and the preceding report.

The native projection inventory contains six audit declarations with `tbody td code`; the repaired projection retains both ancestors for all six. Other native nested selectors are preserved by complete scoped DOM/response comparisons. Single-node icon, SVG text and public-link selectors retain their original tag/attributes/text checks. Controller `assert_select` is structural: hidden-stack cases keep their explicit hidden attributes and empty descendants.

The browser count inventory was checked against each original visibility option. Default-visible picker rows, member minimums and identity selectors now use rendered visibility. Starred/Online/Offline grouping and hidden profile-card probes explicitly use `visible: :all` in Rails and keep attached-node semantics. Menu dismissal counts all rendered menus, while the intermediate open-menu assertion keeps Rails’ explicit attached `:not([hidden])` predicate. The existing scoped popover, directory bar, picker bar, room title and timeline-note locators preserve their selector ancestry.

| Record | Current Rails assertion count | Ancestry / cardinality disposition |
| --- | ---: | --- |
| P0239 | 6 | Fixed: `tbody → td → code`; malformed th/code producer rejected. Other table predicates retain tbody ancestry and exact projected cardinality. |
| P0240 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0241 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0242 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0243 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0244 | 3 | Actual scoped DOM, complete response bytes, or exact single-tag/attribute/text projection preserves original selector and cardinality. |
| P0245 | 5 | Fixed: `tbody → td → code`; malformed th/code producer rejected. Other table predicates retain tbody ancestry and exact projected cardinality. |
| P0246 | 3 | Fixed: `tbody → td → code`; malformed th/code producer rejected. Other table predicates retain tbody ancestry and exact projected cardinality. |
| P0247 | 5 | Fixed: `tbody → td → code`; malformed th/code producer rejected. Other table predicates retain tbody ancestry and exact projected cardinality. |
| P0248 | 5 | Actual scoped DOM, complete response bytes, or exact single-tag/attribute/text projection preserves original selector and cardinality. |
| P0249 | 10 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0250 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0253 | 2 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0230 | 3 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0231 | 3 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0232 | 2 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0254 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0255 | 3 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0256 | 2 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0263 | 2 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0264 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0214 | 4 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0215 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0219 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0220 | 5 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0221 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0216 | 3 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0217 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0218 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0233 | 5 | Actual scoped DOM, complete response bytes, or exact single-tag/attribute/text projection preserves original selector and cardinality. |
| P0234 | 5 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0088 | 7 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0238 | 4 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0235 | 3 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0236 | 3 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0237 | 2 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0265 | 8 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0266 | 5 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0267 | 2 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0268 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0269 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0270 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0271 | 2 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0257 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0258 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0259 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0260 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0261 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0262 | 2 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0085 | 2 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0072 | 6 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0073 | 2 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0074 | 2 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0075 | 4 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0076 | 4 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0077 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0078 | 4 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0079 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0080 | 4 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0081 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0082 | 5 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0083 | 4 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0084 | 5 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0086 | 6 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0087 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0278 | 5 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0279 | 3 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0280 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0281 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0282 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0283 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0284 | 2 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0285 | 2 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0286 | 2 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0203 | 1 | Actual scoped DOM, complete response bytes, or exact single-tag/attribute/text projection preserves original selector and cardinality. |
| P0206 | 1 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0102 | 5 | Actual scoped DOM, complete response bytes, or exact single-tag/attribute/text projection preserves original selector and cardinality. |
| P0176 | 2 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0186 | 2 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0154 | 2 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0165 | 2 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0190 | 3 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0191 | 3 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| S0054 | 6 | Complete scoped live/quiet/cache/group DOM retains every nested tag, attribute, text and count; unconfigured zero selectors preserve section/controller-token scope. |
| S0073 | 6 | Complete scoped live/quiet/cache/group DOM retains every nested tag, attribute, text and count; unconfigured zero selectors preserve section/controller-token scope. |
| S0092 | 5 | Complete scoped live/quiet/cache/group DOM retains every nested tag, attribute, text and count; unconfigured zero selectors preserve section/controller-token scope. |
| S0106 | 5 | Complete scoped live/quiet/cache/group DOM retains every nested tag, attribute, text and count; unconfigured zero selectors preserve section/controller-token scope. |
| S0119 | 9 | Complete scoped live/quiet/cache/group DOM retains every nested tag, attribute, text and count; unconfigured zero selectors preserve section/controller-token scope. |
| S0147 | 5 | Complete scoped live/quiet/cache/group DOM retains every nested tag, attribute, text and count; unconfigured zero selectors preserve section/controller-token scope. |
| S0181 | 4 | Complete scoped live/quiet/cache/group DOM retains every nested tag, attribute, text and count; unconfigured zero selectors preserve section/controller-token scope. |
| S0193 | 4 | Complete scoped live/quiet/cache/group DOM retains every nested tag, attribute, text and count; unconfigured zero selectors preserve section/controller-token scope. |
| P0251 | 5 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0252 | 5 | No default-visible selector/count to weaken; actual status, redirects, saved state, audits, headers/bytes, or query/queue predicates retained. |
| P0354 | 5 | Scoped selector/visible text/focus/current-path predicates checked against pin; no further ancestry or visibility-count gap found. |
| P0355 | 5 | Scoped selector/visible text/focus/current-path predicates checked against pin; no further ancestry or visibility-count gap found. |
| P0356 | 2 | Scoped selector/visible text/focus/current-path predicates checked against pin; no further ancestry or visibility-count gap found. |
| P0357 | 2 | Scoped selector/visible text/focus/current-path predicates checked against pin; no further ancestry or visibility-count gap found. |
| P0358 | 3 | Scoped selector/visible text/focus/current-path predicates checked against pin; no further ancestry or visibility-count gap found. |
| P0359 | 4 | Fixed minimum: wait for at least three visible rows at Rails’ explicit 10-second deadline. |
| P0360 | 2 | Scoped selector/visible text/focus/current-path predicates checked against pin; no further ancestry or visibility-count gap found. |
| P0361 | 3 | Scoped selector/visible text/focus/current-path predicates checked against pin; no further ancestry or visibility-count gap found. |
| P0362 | 3 | Scoped selector/visible text/focus/current-path predicates checked against pin; no further ancestry or visibility-count gap found. |
| P0363 | 3 | Scoped selector/visible text/focus/current-path predicates checked against pin; no further ancestry or visibility-count gap found. |
| P0364 | 10 | Picker scope retained; default-visible row counts and negative visible selectors corrected. Explicit visible:all checked fields remain attached-node probes. |
| P0365 | 5 | Picker scope retained; default-visible row counts and negative visible selectors corrected. Explicit visible:all checked fields remain attached-node probes. |
| P0366 | 6 | Picker scope retained; default-visible row counts and negative visible selectors corrected. Explicit visible:all checked fields remain attached-node probes. |
| P0367 | 3 | Picker scope retained; default-visible row counts and negative visible selectors corrected. Explicit visible:all checked fields remain attached-node probes. |
| P0368 | 4 | Picker scope retained; default-visible row counts and negative visible selectors corrected. Explicit visible:all checked fields remain attached-node probes. |
| P0369 | 11 | Added visible rename notice and actual departure from edit URL; show-URL exclusion and actual persisted membership check retained. |
| P0371 | 6 | Fixed minimum: wait for at least three visible rows at Rails’ explicit 10-second deadline. |
| P0372 | 6 | Fixed minimum: wait for at least three visible rows at Rails’ explicit 10-second deadline. |
| P0300 | 18 | All current-pin predicates present: actual DB stamp before restart/navigation, false auto-start attribute, visible/hidden card and help/step predicates. |
| P0301 | 5 | All current-pin predicates present: actual DB stamp before restart/navigation, false auto-start attribute, visible/hidden card and help/step predicates. |
| P0302 | 4 | All current-pin predicates present: actual DB stamp before restart/navigation, false auto-start attribute, visible/hidden card and help/step predicates. |
| P0303 | 3 | All current-pin predicates present: actual DB stamp before restart/navigation, false auto-start attribute, visible/hidden card and help/step predicates. |
| P0400 | 7 | Rendered menu dismissal and identity existence corrected; explicit visible:all grouping/hidden-card selectors retained. |
| P0401 | 14 | Rendered menu dismissal and identity existence corrected; explicit visible:all grouping/hidden-card selectors retained. |
| P0402 | 5 | Rendered menu dismissal and identity existence corrected; explicit visible:all grouping/hidden-card selectors retained. |
| P0403 | 6 | Rendered menu dismissal and identity existence corrected; explicit visible:all grouping/hidden-card selectors retained. |
| P0276 | 2 | Scoped selector/visible text/focus/current-path predicates checked against pin; no further ancestry or visibility-count gap found. |
