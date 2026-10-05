# PR #244 per-assertion audit

Original declarations and runtime fixtures use Rails 78b9b1546bdab4c6c1c9b8ddb94512f661289112.

Every row names the original assertion and the actual Rust check. Whole-byte/DOM checks retain tags, attributes, text and cardinality; their fixture cases are named below. Reopened gaps are explicit and retain the previous insufficient claim in JSON history.

## P0089: about renders signed-out with stable title and navigation

Status: **closed**. Native identities:

- `controllers::public_pages::tests::public_page_bodies_match_rails_with_operator_escaping_and_email_uri_encoding`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/public_pages_controller_test.rb:25` — `assert_response :success` | rust/crates/campfire/src/controllers/public_pages.rs:198 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:26` — `assert_select "title", "Smartfire &#124; About"` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:27` — `assert_select "h1", "About Smartfire"` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:28` — `assert_select 'nav[aria-label="Public pages"] a[href="/about"]', minimum: 1` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:29` — `assert_select 'a[href="/privacy"]', minimum: 1` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:30` — `assert_select 'a[href="/terms"]', minimum: 1` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:31` — `assert_select 'a[href="/session/new"]', minimum: 1` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:32` — `assert_match(/self-hosted/i, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |

## P0090: privacy renders signed-out with Google disclosures

Status: **closed**. Native identities:

- `controllers::public_pages::tests::public_page_bodies_match_rails_with_operator_escaping_and_email_uri_encoding`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/public_pages_controller_test.rb:38` — `assert_response :success` | rust/crates/campfire/src/controllers/public_pages.rs:198 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:39` — `assert_select "title", "Smartfire &#124; Privacy Policy"` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:40` — `assert_select "h1", "Privacy Policy"` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:41` — `assert_match(/September 18, 2026/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:42` — `assert_match(/openid email profile/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:43` — `assert_match(/myaccount\.google\.com\/connections/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:44` — `assert_match(/developers\.google\.com\/terms\/api-services-user-data-policy/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:45` — `assert_match(/direct messages/i, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |

## P0091: privacy discloses picker-only Drive previews

Status: **closed**. Native identities:

- `controllers::public_pages::tests::public_page_bodies_match_rails_with_operator_escaping_and_email_uri_encoding`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/public_pages_controller_test.rb:51` — `assert_response :success` | rust/crates/campfire/src/controllers/public_pages.rs:198 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:52` — `assert_match(/per-file consent/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:53` — `assert_match(/never your whole Drive/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:54` — `assert_match(/plain chip/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:55` — `assert_no_match(/metadata\.readonly/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |

## P0092: terms renders signed-out with software-license framing

Status: **closed**. Native identities:

- `controllers::public_pages::tests::public_page_bodies_match_rails_with_operator_escaping_and_email_uri_encoding`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/public_pages_controller_test.rb:61` — `assert_response :success` | rust/crates/campfire/src/controllers/public_pages.rs:198 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:62` — `assert_select "title", "Smartfire &#124; Terms of Service"` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:63` — `assert_select "h1", "Terms of Service"` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:64` — `assert_match(/MIT-LICENSE/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:65` — `assert_match(/September 18, 2026/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |

## P0093: public pages never redirect to sign-in and set no session cookie

Status: **closed**. Native identities:

- `controllers::public_pages::tests::public_pages_bypass_authentication_browser_and_private_state`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/public_pages_controller_test.rb:72` — `assert_response :success, "expected success for #{url}"` | rust/crates/campfire/src/controllers/public_pages.rs:99 | Real anonymous/crawler/old-browser requests and signed-in HEAD; exact success/empty body/no cookie and private-state absence. |
| `test/controllers/public_pages_controller_test.rb:73` — `assert_no_match(/session_token/, response.headers["Set-Cookie"].to_s)` | rust/crates/campfire/src/controllers/public_pages.rs:100 | Real anonymous/crawler/old-browser requests and signed-in HEAD; exact success/empty body/no cookie and private-state absence. |

## P0094: public pages ignore the modern-browser gate, even for crawlers

Status: **closed**. Native identities:

- `controllers::public_pages::tests::public_pages_bypass_authentication_browser_and_private_state`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/public_pages_controller_test.rb:82` — `assert_response :success, "expected success for #{url} with #{user_agent.inspect}"` | rust/crates/campfire/src/controllers/public_pages.rs:99 | Real anonymous/crawler/old-browser requests and signed-in HEAD; exact success/empty body/no cookie and private-state absence. |
| `test/controllers/public_pages_controller_test.rb:83` — `assert_select "h1", text: /Upgrade to a supported web browser/, count: 0` | rust/crates/campfire/src/controllers/public_pages.rs:120 | Real anonymous/crawler/old-browser requests and signed-in HEAD; exact success/empty body/no cookie and private-state absence. |

## P0095: public pages render without OAuth configured

Status: **closed**. Native identities:

- `controllers::public_pages::tests::public_pages_bypass_authentication_browser_and_private_state`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/public_pages_controller_test.rb:89` — `assert_not Google::Client.configured?` | rust/crates/campfire/src/controllers/public_pages.rs:86 | Actual injected application Google client/sign-in configuration is explicitly asserted unconfigured before the real requests. |
| `test/controllers/public_pages_controller_test.rb:90` — `assert_not Google::SignIn.configured?` | rust/crates/campfire/src/controllers/public_pages.rs:87 | Actual injected application Google client/sign-in configuration is explicitly asserted unconfigured before the real requests. |
| `test/controllers/public_pages_controller_test.rb:94` — `assert_response :success, "expected success for #{url} without OAuth"` | rust/crates/campfire/src/controllers/public_pages.rs:99 | Real anonymous/crawler/old-browser requests and signed-in HEAD; exact success/empty body/no cookie and private-state absence. |

## P0096: public pages disclose no private state, credentials, or scripts

Status: **closed**. Native identities:

- `controllers::public_pages::tests::public_pages_bypass_authentication_browser_and_private_state`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/public_pages_controller_test.rb:100` — `assert_response :success` | rust/crates/campfire/src/controllers/public_pages.rs:99; rust/crates/campfire/src/controllers/public_pages.rs:131 | Real anonymous/crawler/old-browser requests and signed-in HEAD; exact success/empty body/no cookie and private-state absence. |
| `test/controllers/public_pages_controller_test.rb:103` — `assert_no_match(/david@37signals\.com/, body)` | rust/crates/campfire/src/controllers/public_pages.rs:120; rust/crates/campfire/src/controllers/public_pages.rs:131 | Real anonymous/crawler/old-browser requests and signed-in HEAD; exact success/empty body/no cookie and private-state absence. |
| `test/controllers/public_pages_controller_test.rb:104` — `assert_no_match(/current-user-id/, body)` | rust/crates/campfire/src/controllers/public_pages.rs:120; rust/crates/campfire/src/controllers/public_pages.rs:131 | Real anonymous/crawler/old-browser requests and signed-in HEAD; exact success/empty body/no cookie and private-state absence. |
| `test/controllers/public_pages_controller_test.rb:105` — `assert_no_match(/vapid-public-key/, body)` | rust/crates/campfire/src/controllers/public_pages.rs:120; rust/crates/campfire/src/controllers/public_pages.rs:131 | Real anonymous/crawler/old-browser requests and signed-in HEAD; exact success/empty body/no cookie and private-state absence. |
| `test/controllers/public_pages_controller_test.rb:106` — `assert_no_match(/google-picker-client-id/, body)` | rust/crates/campfire/src/controllers/public_pages.rs:120; rust/crates/campfire/src/controllers/public_pages.rs:131 | Real anonymous/crawler/old-browser requests and signed-in HEAD; exact success/empty body/no cookie and private-state absence. |
| `test/controllers/public_pages_controller_test.rb:107` — `assert_no_match(/google-drive-previews/, body)` | rust/crates/campfire/src/controllers/public_pages.rs:120; rust/crates/campfire/src/controllers/public_pages.rs:131 | Real anonymous/crawler/old-browser requests and signed-in HEAD; exact success/empty body/no cookie and private-state absence. |
| `test/controllers/public_pages_controller_test.rb:108` — `assert_no_match(/brand-icon-names/, body)` | rust/crates/campfire/src/controllers/public_pages.rs:120; rust/crates/campfire/src/controllers/public_pages.rs:131 | Real anonymous/crawler/old-browser requests and signed-in HEAD; exact success/empty body/no cookie and private-state absence. |
| `test/controllers/public_pages_controller_test.rb:109` — `assert_no_match(/<script/, body)` | rust/crates/campfire/src/controllers/public_pages.rs:120; rust/crates/campfire/src/controllers/public_pages.rs:131 | Real anonymous/crawler/old-browser requests and signed-in HEAD; exact success/empty body/no cookie and private-state absence. |
| `test/controllers/public_pages_controller_test.rb:110` — `assert_no_match(/importmap/, body)` | rust/crates/campfire/src/controllers/public_pages.rs:120; rust/crates/campfire/src/controllers/public_pages.rs:131 | Real anonymous/crawler/old-browser requests and signed-in HEAD; exact success/empty body/no cookie and private-state absence. |
| `test/controllers/public_pages_controller_test.rb:111` — `assert_no_match(/action-cable&#124;turbo-prefetch/, body)` | rust/crates/campfire/src/controllers/public_pages.rs:120; rust/crates/campfire/src/controllers/public_pages.rs:131 | Real anonymous/crawler/old-browser requests and signed-in HEAD; exact success/empty body/no cookie and private-state absence. |
| `test/controllers/public_pages_controller_test.rb:112` — `assert_no_match(/noindex/, body)` | rust/crates/campfire/src/controllers/public_pages.rs:120; rust/crates/campfire/src/controllers/public_pages.rs:131 | Real anonymous/crawler/old-browser requests and signed-in HEAD; exact success/empty body/no cookie and private-state absence. |
| `test/controllers/public_pages_controller_test.rb:113` — `assert_no_match(/csrf-token&#124;csrf-param/, body)` | rust/crates/campfire/src/controllers/public_pages.rs:120; rust/crates/campfire/src/controllers/public_pages.rs:131 | Real anonymous/crawler/old-browser requests and signed-in HEAD; exact success/empty body/no cookie and private-state absence. |

## P0097: public pages allow zoom and honor color schemes

Status: **closed**. Native identities:

- `controllers::public_pages::tests::public_page_bodies_match_rails_with_operator_escaping_and_email_uri_encoding`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/public_pages_controller_test.rb:118` — `assert_response :success` | rust/crates/campfire/src/controllers/public_pages.rs:198 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:120` — `assert_select 'meta[name="viewport"][content="width=device-width, initial-scale=1"]'` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:121` — `assert_select 'meta[name="color-scheme"][content="light dark"]'` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |

## P0098: non-HTML formats expose nothing

Status: **closed**. Native identities:

- `controllers::public_pages::tests::public_pages_are_html_only_and_allow_wildcard_accept`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/public_pages_controller_test.rb:126` — `assert_response :not_found` | rust/crates/campfire/src/controllers/public_pages.rs:161 | Actual suffix and Accept requests: HTML/wildcard successful, others empty 404. |
| `test/controllers/public_pages_controller_test.rb:129` — `assert_response :not_found` | rust/crates/campfire/src/controllers/public_pages.rs:161 | Actual suffix and Accept requests: HTML/wildcard successful, others empty 404. |
| `test/controllers/public_pages_controller_test.rb:132` — `assert_response :not_found` | rust/crates/campfire/src/controllers/public_pages.rs:161 | Actual suffix and Accept requests: HTML/wildcard successful, others empty 404. |
| `test/controllers/public_pages_controller_test.rb:135` — `assert_response :not_found` | rust/crates/campfire/src/controllers/public_pages.rs:161 | Actual suffix and Accept requests: HTML/wildcard successful, others empty 404. |

## P0099: wildcard Accept header receives the HTML page

Status: **closed**. Native identities:

- `controllers::public_pages::tests::public_pages_are_html_only_and_allow_wildcard_accept`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/public_pages_controller_test.rb:142` — `assert_response :success, "expected success for #{url} with Accept: */*"` | rust/crates/campfire/src/controllers/public_pages.rs:149 | Actual suffix and Accept requests: HTML/wildcard successful, others empty 404. |
| `test/controllers/public_pages_controller_test.rb:143` — `assert_match(/<title>Smartfire/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:150 | Actual suffix and Accept requests: HTML/wildcard successful, others empty 404. |

## P0100: HEAD requests succeed

Status: **closed**. Native identities:

- `controllers::public_pages::tests::public_pages_accept_anonymous_head_without_a_body`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/public_pages_controller_test.rb:150` — `assert_response :success, "expected HEAD success for #{url}"` | rust/crates/campfire/src/controllers/public_pages.rs:239 | Cookie-free real HEAD at all three URLs; status 200 and zero body bytes. The existing authenticated HEAD variant remains separately tested. Cases: anonymous HEAD /about /privacy /terms. |
| `test/controllers/public_pages_controller_test.rb:151` — `assert_empty response.body` | rust/crates/campfire/src/controllers/public_pages.rs:240 | Cookie-free real HEAD at all three URLs; status 200 and zero body bytes. The existing authenticated HEAD variant remains separately tested. Cases: anonymous HEAD /about /privacy /terms. |

## P0101: unconfigured installation uses generic wording without env names

Status: **closed**. Native identities:

- `controllers::public_pages::tests::public_page_bodies_match_rails_with_operator_escaping_and_email_uri_encoding`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/public_pages_controller_test.rb:157` — `assert_response :success` | rust/crates/campfire/src/controllers/public_pages.rs:198 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:159` — `assert_match(/organization hosting/i, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:160` — `assert_match(/workspace administrator/i, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:161` — `assert_no_match(/LEGAL_OPERATOR_NAME/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:162` — `assert_no_match(/LEGAL_CONTACT_EMAIL/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:163` — `assert_no_match(/mailto:/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |

## P0102: configured installation names the operator and contact

Status: **closed**. Native identities:

- `controllers::public_pages::tests::public_page_bodies_match_rails_with_operator_escaping_and_email_uri_encoding`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/public_pages_controller_test.rb:171` — `assert_response :success` | rust/crates/campfire/src/controllers/public_pages.rs:198 | Actual anonymous public-page status and complete bytes equal Rails with the original Acme Widgets/privacy@example.com environment, including the mailto anchor and both privacy/About bodies. The panic occurs only when byte equality fails. Cases: original_configured. |
| `test/controllers/public_pages_controller_test.rb:173` — `assert_match(/Acme Widgets/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:213 | Actual anonymous public-page status and complete bytes equal Rails with the original Acme Widgets/privacy@example.com environment, including the mailto anchor and both privacy/About bodies. The panic occurs only when byte equality fails. Cases: original_configured. |
| `test/controllers/public_pages_controller_test.rb:174` — `assert_select 'a[href="mailto:privacy@example.com"]'` | rust/crates/campfire/src/controllers/public_pages.rs:213 | Actual anonymous public-page status and complete bytes equal Rails with the original Acme Widgets/privacy@example.com environment, including the mailto anchor and both privacy/About bodies. The panic occurs only when byte equality fails. Cases: original_configured. |
| `test/controllers/public_pages_controller_test.rb:177` — `assert_response :success` | rust/crates/campfire/src/controllers/public_pages.rs:198 | Actual anonymous public-page status and complete bytes equal Rails with the original Acme Widgets/privacy@example.com environment, including the mailto anchor and both privacy/About bodies. The panic occurs only when byte equality fails. Cases: original_configured. |
| `test/controllers/public_pages_controller_test.rb:178` — `assert_match(/Acme Widgets/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:213 | Actual anonymous public-page status and complete bytes equal Rails with the original Acme Widgets/privacy@example.com environment, including the mailto anchor and both privacy/About bodies. The panic occurs only when byte equality fails. Cases: original_configured. |

## P0103: operator name is escaped and malicious contact email is dropped

Status: **closed**. Native identities:

- `controllers::public_pages::tests::public_page_bodies_match_rails_with_operator_escaping_and_email_uri_encoding`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/public_pages_controller_test.rb:186` — `assert_response :success` | rust/crates/campfire/src/controllers/public_pages.rs:198 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: hostile. |
| `test/controllers/public_pages_controller_test.rb:188` — `assert_no_match(/<script>alert\(1\)<\/script>/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: hostile. |
| `test/controllers/public_pages_controller_test.rb:189` — `assert_match(/&lt;script&gt;alert\(1\)&lt;\/script&gt;/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: hostile. |
| `test/controllers/public_pages_controller_test.rb:190` — `assert_no_match(/mailto:/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: hostile. |
| `test/controllers/public_pages_controller_test.rb:191` — `assert_no_match(/LEGAL_CONTACT_EMAIL/, response.body)` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: hostile. |

## P0104: public pages open in a new tab so following them never disturbs the current tab

Status: **closed**. Native identities:

- `controllers::public_pages::tests::public_page_bodies_match_rails_with_operator_escaping_and_email_uri_encoding`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/public_pages_controller_test.rb:197` — `assert_response :success` | rust/crates/campfire/src/controllers/public_pages.rs:198 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:198` — `assert_select 'nav[aria-label="Public pages"] a[target="_blank"][rel="noopener"]', count: 3` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:199` — `assert_select 'nav[aria-label="Footer"] a[target="_blank"][rel="noopener"]', count: 3` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:200` — `assert_select 'a.public-nav__signin[target="_blank"]', count: 0` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:201` — `assert_select 'main a[href="/privacy"][target="_blank"]'` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |
| `test/controllers/public_pages_controller_test.rb:202` — `assert_select 'main a[href="/terms"][target="_blank"]'` | rust/crates/campfire/src/controllers/public_pages.rs:201 | Full actual public page bytes incl title/headings/navigation/text match pinned Rails. Cases: default. |

## P0105: sign-in page links the public pages without OAuth

Status: **closed**. Native identities:

- `controllers::public_pages::tests::unconfigured_sign_in_links_all_public_pages_in_new_tabs`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/public_pages_controller_test.rb:207` — `assert_response :success` | rust/crates/campfire/src/controllers/public_pages.rs:75 | Real anonymous sign-in GET with no OAuth; exact new-tab/rel public links. |
| `test/controllers/public_pages_controller_test.rb:209` — `assert_select 'nav[aria-label="About this workspace"] a[href="/about"][target="_blank"][rel="noopener"]'` | rust/crates/campfire/src/controllers/public_pages.rs:66; rust/crates/campfire/src/controllers/public_pages.rs:59 | Real anonymous sign-in GET with no OAuth; exact new-tab/rel public links. |
| `test/controllers/public_pages_controller_test.rb:210` — `assert_select 'nav[aria-label="About this workspace"] a[href="/privacy"][target="_blank"][rel="noopener"]'` | rust/crates/campfire/src/controllers/public_pages.rs:66; rust/crates/campfire/src/controllers/public_pages.rs:59 | Real anonymous sign-in GET with no OAuth; exact new-tab/rel public links. |
| `test/controllers/public_pages_controller_test.rb:211` — `assert_select 'nav[aria-label="About this workspace"] a[href="/terms"][target="_blank"][rel="noopener"]'` | rust/crates/campfire/src/controllers/public_pages.rs:66; rust/crates/campfire/src/controllers/public_pages.rs:59 | Real anonymous sign-in GET with no OAuth; exact new-tab/rel public links. |

## P0106: sign-in page keeps public links beside Google sign-in when configured

Status: **closed**. Native identities:

- `controllers::public_pages::sign_in_google_tests::configured_sign_in_keeps_public_links_and_matches_complete_rails_bodies`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/public_pages_controller_test.rb:219` — `assert_response :success` | rust/crates/campfire/src/controllers/public_pages/sign_in_google_tests.rs:61 | Real configured sign-in GET with Google button and all public links plus complete pinned sign-in fragment bytes. |
| `test/controllers/public_pages_controller_test.rb:221` — `assert_match(/Sign in with Google/, response.body)` | rust/crates/campfire/src/controllers/public_pages/sign_in_google_tests.rs:65 | Real configured sign-in GET with Google button and all public links plus complete pinned sign-in fragment bytes. |
| `test/controllers/public_pages_controller_test.rb:222` — `assert_select 'nav[aria-label="About this workspace"] a[href="/about"]'` | rust/crates/campfire/src/controllers/public_pages.rs:66; rust/crates/campfire/src/controllers/public_pages.rs:59 | Real configured sign-in GET with Google button and all public links plus complete pinned sign-in fragment bytes. |
| `test/controllers/public_pages_controller_test.rb:223` — `assert_select 'nav[aria-label="About this workspace"] a[href="/privacy"]'` | rust/crates/campfire/src/controllers/public_pages.rs:66; rust/crates/campfire/src/controllers/public_pages.rs:59 | Real configured sign-in GET with Google button and all public links plus complete pinned sign-in fragment bytes. |
| `test/controllers/public_pages_controller_test.rb:224` — `assert_select 'nav[aria-label="About this workspace"] a[href="/terms"]'` | rust/crates/campfire/src/controllers/public_pages.rs:66; rust/crates/campfire/src/controllers/public_pages.rs:59 | Real configured sign-in GET with Google button and all public links plus complete pinned sign-in fragment bytes. |

## P0113: show

Status: **closed**. Native identities:

- `controllers::users::people_tests::own_public_profile_matches_the_original_show_request`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:11` — `assert_response :ok` | rust/crates/campfire/src/controllers/users/people_tests.rs:451 | Real authenticated David GET of his own public profile, matching the original show request; peer and bot Message-button branches retain separate tests. Cases: David self GET. |

## P0114: profile message buttons carry the accessible name

Status: **closed**. Native identities:

- `controllers::users::people_tests::profile_message_buttons_carry_the_accessible_name`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:18` — `assert_response :ok` | rust/crates/campfire/src/controllers/users/people_tests.rs:406 | Real human and bot GET; status, exact accessible Message button count, human ban button and no aria-label on images. |
| `test/controllers/users_controller_test.rb:19` — `assert_select "button[aria-label='Message Kevin']", 1` | rust/crates/campfire/src/controllers/users/people_tests.rs:418 | Real human and bot GET; status, exact accessible Message button count, human ban button and no aria-label on images. |
| `test/controllers/users_controller_test.rb:20` — `assert_select "button", text: "Ban Kevin"` | rust/crates/campfire/src/controllers/users/people_tests.rs:424 | Real human and bot GET; status, exact accessible Message button count, human ban button and no aria-label on images. |
| `test/controllers/users_controller_test.rb:21` — `assert_select "img[aria-label]", 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:438 | Real human and bot GET; status, exact accessible Message button count, human ban button and no aria-label on images. |
| `test/controllers/users_controller_test.rb:24` — `assert_response :ok` | rust/crates/campfire/src/controllers/users/people_tests.rs:406 | Real human and bot GET; status, exact accessible Message button count, human ban button and no aria-label on images. |
| `test/controllers/users_controller_test.rb:25` — `assert_select "button[aria-label='Message Bender Bot']", 1` | rust/crates/campfire/src/controllers/users/people_tests.rs:418 | Real human and bot GET; status, exact accessible Message button count, human ban button and no aria-label on images. |
| `test/controllers/users_controller_test.rb:26` — `assert_select "img[aria-label]", 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:438 | Real human and bot GET; status, exact accessible Message button count, human ban button and no aria-label on images. |

## P0115: bot profile links to capability grants for admins

Status: **closed**. Native identities:

- `controllers::users::agent_profile_tests::admin_grants`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:34` — `assert_response :ok` | rust/crates/campfire/src/controllers/users/agent_profile_tests.rs:104 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: admin_grants. |
| `test/controllers/users_controller_test.rb:35` — `assert_select "a[href='#{account_bot_grants_path(users(:bender))}']", 1` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: admin_grants. |

## P0116: bot profile links to capability grants for the agent owner

Status: **closed**. Native identities:

- `controllers::users::agent_profile_tests::owner_grants`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:44` — `assert_response :ok` | rust/crates/campfire/src/controllers/users/agent_profile_tests.rs:104 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: owner_grants. |
| `test/controllers/users_controller_test.rb:45` — `assert_select "a[href='#{account_bot_grants_path(users(:bender))}']", 1` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: owner_grants. |

## P0117: bot profile hides capability grants from anyone else

Status: **closed**. Native identities:

- `controllers::users::agent_profile_tests::peer_hides_grants`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:53` — `assert_response :ok` | rust/crates/campfire/src/controllers/users/agent_profile_tests.rs:104 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: peer_hides_grants. |
| `test/controllers/users_controller_test.rb:54` — `assert_select "a[href='#{account_bot_grants_path(users(:bender))}']", 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: peer_hides_grants. |

## P0118: bot profile shows agent identity, status, rooms, and grants to a member

Status: **closed**. Native identities:

- `controllers::users::agent_profile_tests::original_identity_and_two_room_visibility_match_rails`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:66` — `assert_response :ok` | rust/crates/campfire/src/controllers/users/agent_profile_tests.rs:104 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: original_identity. |
| `test/controllers/users_controller_test.rb:67` — `assert_select "h1", text: "Bender Bot"` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: original_identity. |
| `test/controllers/users_controller_test.rb:68` — `assert_match "Workspace agent, managed by David", response.body` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: original_identity. |
| `test/controllers/users_controller_test.rb:69` — `assert_match "OpenAI · Codex CLI 0.9", response.body` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: original_identity. |
| `test/controllers/users_controller_test.rb:70` — `assert_match "Does things", response.body` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: original_identity. |
| `test/controllers/users_controller_test.rb:71` — `assert_select ".agent-status-badge--working", text: "Working"` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: original_identity. |
| `test/controllers/users_controller_test.rb:72` — `assert_select ".agent-status-note", text: "on it"` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: original_identity. |
| `test/controllers/users_controller_test.rb:73` — `assert_match(/since .* ago/, response.body)` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: original_identity. |
| `test/controllers/users_controller_test.rb:74` — `assert_match "never", response.body` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: original_identity. |
| `test/controllers/users_controller_test.rb:75` — `assert_select "a[href='#{room_path(rooms(:bender_and_kevin))}']", 1` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: original_identity. |
| `test/controllers/users_controller_test.rb:76` — `assert_select "a[href='#{room_path(rooms(:watercooler))}']", 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: original_identity. |
| `test/controllers/users_controller_test.rb:77` — `assert_match "and 1 more", response.body` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: original_identity. |
| `test/controllers/users_controller_test.rb:78` — `assert_match "legacy access (no grants recorded)", response.body` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: original_identity. |

## P0119: bot profile shows the 24-hour activity line to the owner

Status: **closed**. Native identities:

- `controllers::users::agent_profile_tests::owner_activity`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:90` — `assert_response :ok` | rust/crates/campfire/src/controllers/users/agent_profile_tests.rs:104 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: owner_activity. |
| `test/controllers/users_controller_test.rb:91` — `assert_match "Last 24 hours: 1 delivered, 0 acknowledged, 0 posted, 0 suppressed", response.body` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: owner_activity. |

## P0120: bot profile shows the 24-hour activity line to an admin

Status: **closed**. Native identities:

- `controllers::users::agent_profile_tests::admin_activity`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:100` — `assert_response :ok` | rust/crates/campfire/src/controllers/users/agent_profile_tests.rs:104 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: admin_activity. |
| `test/controllers/users_controller_test.rb:101` — `assert_match "Last 24 hours:", response.body` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: admin_activity. |

## P0121: bot profile hides the 24-hour activity line from another member

Status: **closed**. Native identities:

- `controllers::users::agent_profile_tests::peer_hides_activity`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:109` — `assert_response :ok` | rust/crates/campfire/src/controllers/users/agent_profile_tests.rs:104 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: peer_hides_activity. |
| `test/controllers/users_controller_test.rb:110` — `assert_no_match "Last 24 hours:", response.body` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: peer_hides_activity. |

## P0122: bot profile hides rooms the viewer is not a member of

Status: **closed**. Native identities:

- `controllers::users::agent_profile_tests::original_nonmember_hides_both_rooms_match_rails`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:118` — `assert_response :ok` | rust/crates/campfire/src/controllers/users/agent_profile_tests.rs:104 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: original_private_rooms. |
| `test/controllers/users_controller_test.rb:119` — `assert_select "a[href='#{room_path(rooms(:watercooler))}']", 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: original_private_rooms. |
| `test/controllers/users_controller_test.rb:120` — `assert_select "a[href='#{room_path(rooms(:bender_and_kevin))}']", 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: original_private_rooms. |
| `test/controllers/users_controller_test.rb:121` — `assert_match "and 2 more", response.body` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: original_private_rooms. |

## P0123: suspended agent profile shows Suspended

Status: **closed**. Native identities:

- `controllers::users::agent_profile_tests::suspended`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:130` — `assert_response :ok` | rust/crates/campfire/src/controllers/users/agent_profile_tests.rs:104 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: suspended. |
| `test/controllers/users_controller_test.rb:131` — `assert_select ".agent-status-badge--suspended", text: "Suspended"` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: suspended. |

## P0124: bot without an agent keeps the minimal profile

Status: **closed**. Native identities:

- `controllers::users::agent_profile_tests::minimal_bot`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:140` — `assert_response :ok` | rust/crates/campfire/src/controllers/users/agent_profile_tests.rs:104 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: minimal_bot. |
| `test/controllers/users_controller_test.rb:141` — `assert_select "h1", text: "Bender Bot"` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: minimal_bot. |
| `test/controllers/users_controller_test.rb:142` — `assert_select ".agent-status", 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: minimal_bot. |
| `test/controllers/users_controller_test.rb:143` — `assert_no_match "Workspace agent", response.body` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual viewer GET; complete routed profile-section DOM equals the pinned full-byte fragment oracle. Every tag, text, link/action, status class, room count and management control is retained; only the randomized CSRF value is excluded from the additional DOM comparison. Separate detached comparison checks every original fragment byte. Cases: minimal_bot. |

## P0125: new

Status: **closed**. Native identities:

- `controllers::users::joining_tests::join_page_matches_complete_rails_body_and_access_checks`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:148` — `assert_response :success` | rust/crates/campfire/src/controllers/users/joining_tests.rs:55 | Real anonymous valid/invalid-code GET and signed-in GET; exact statuses and root redirect, plus complete signup byte oracle. Cases: valid_get, signed_get. |

## P0126: new does not allow a signed in user

Status: **closed**. Native identities:

- `controllers::users::joining_tests::join_page_matches_complete_rails_body_and_access_checks`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:155` — `assert_redirected_to root_url` | rust/crates/campfire/src/controllers/users/joining_tests.rs:63 | Real anonymous valid/invalid-code GET and signed-in GET; exact statuses and root redirect, plus complete signup byte oracle. Cases: valid_get, signed_get. |

## P0127: new requires a join code

Status: **closed**. Native identities:

- `controllers::users::joining_tests::join_page_matches_complete_rails_body_and_access_checks`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:160` — `assert_response :not_found` | rust/crates/campfire/src/controllers/users/joining_tests.rs:59 | Real anonymous valid/invalid-code GET and signed-in GET; exact statuses and root redirect, plus complete signup byte oracle. Cases: valid_get, signed_get. |

## P0128: create

Status: **closed**. Native identities:

- `controllers::users::joining_tests::join_writes_match_rails_duplicate_scope_open_rooms_and_sessions`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:164` — `assert_difference -> { User.count }, 1 do` | rust/crates/campfire/src/controllers/users/joining_tests.rs:129 | Real anonymous POST; duplicate changes no user and redirects to login, valid adds one member with exactly all open rooms, cookie resolves to its session, signed-in rejoin redirects. WS9 approved drift requires the new unenrolled user to visit 2FA setup before profile. Cases: valid, duplicate. |
| `test/controllers/users_controller_test.rb:168` — `assert_redirected_to root_url` | rust/crates/campfire/src/controllers/users/joining_tests.rs:112 | Real anonymous POST; duplicate changes no user and redirects to login, valid adds one member with exactly all open rooms, cookie resolves to its session, signed-in rejoin redirects. WS9 approved drift requires the new unenrolled user to visit 2FA setup before profile. Cases: valid, duplicate. |
| `test/controllers/users_controller_test.rb:171` — `assert_equal user.id, Session.find_by(token: parsed_cookies.signed[:session_token]).user.id` | rust/crates/campfire/src/controllers/users/joining_tests.rs:145 | Real anonymous POST; duplicate changes no user and redirects to login, valid adds one member with exactly all open rooms, cookie resolves to its session, signed-in rejoin redirects. WS9 approved drift requires the new unenrolled user to visit 2FA setup before profile. Cases: valid, duplicate. |
| `test/controllers/users_controller_test.rb:172` — `assert_equal Rooms::Open.all, user.rooms` | rust/crates/campfire/src/controllers/users/joining_tests.rs:131 | Real anonymous POST; duplicate changes no user and redirects to login, valid adds one member with exactly all open rooms, cookie resolves to its session, signed-in rejoin redirects. WS9 approved drift requires the new unenrolled user to visit 2FA setup before profile. Cases: valid, duplicate. |

## P0129: creating a new user with an existing email address will redirect to login screen

Status: **closed**. Native identities:

- `controllers::users::joining_tests::join_writes_match_rails_duplicate_scope_open_rooms_and_sessions`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:176` — `assert_no_difference -> { User.count } do` | rust/crates/campfire/src/controllers/users/joining_tests.rs:86 | The actual post-duplicate User count is compared to the pre-request count; recording the initial count alone is not the receipt. Cases: valid, duplicate. |
| `test/controllers/users_controller_test.rb:180` — `assert_redirected_to new_session_url(email_address: users(:david).email_address)` | rust/crates/campfire/src/controllers/users/joining_tests.rs:84 | Real anonymous POST; duplicate changes no user and redirects to login, valid adds one member with exactly all open rooms, cookie resolves to its session, signed-in rejoin redirects. WS9 approved drift requires the new unenrolled user to visit 2FA setup before profile. Cases: valid, duplicate. |

## P0130: index lists active members with presence and selection

Status: **closed**. Native identities:

- `controllers::users::people_tests::index_lists_active_members_with_presence_and_selection`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:191` — `assert_response :ok` | rust/crates/campfire/src/controllers/users/people_tests.rs:356 | Actual directory GET; exact active selection/user exclusion, presence/Agent/bar, inactive-row exclusion and minimum rows; ordered/starred complete routed directory DOM and full byte oracle. Anonymous redirect is separately checked. Cases: directories. |
| `test/controllers/users_controller_test.rb:192` — `assert_select ".people-directory__row", minimum: 2` | rust/crates/campfire/src/controllers/users/people_tests.rs:385 | Actual directory GET; exact active selection/user exclusion, presence/Agent/bar, inactive-row exclusion and minimum rows; ordered/starred complete routed directory DOM and full byte oracle. Anonymous redirect is separately checked. Cases: directories. |
| `test/controllers/users_controller_test.rb:193` — `assert_select "input[data-multi-select-target='checkbox'][data-user-id='#{users(:jason).id}']", 1` | rust/crates/campfire/src/controllers/users/people_tests.rs:371 | Actual directory GET; exact active selection/user exclusion, presence/Agent/bar, inactive-row exclusion and minimum rows; ordered/starred complete routed directory DOM and full byte oracle. Anonymous redirect is separately checked. Cases: directories. |
| `test/controllers/users_controller_test.rb:194` — `assert_select "input[data-multi-select-target='checkbox'][data-user-id='#{users(:david).id}']", 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:371 | Actual directory GET; exact active selection/user exclusion, presence/Agent/bar, inactive-row exclusion and minimum rows; ordered/starred complete routed directory DOM and full byte oracle. Anonymous redirect is separately checked. Cases: directories. |
| `test/controllers/users_controller_test.rb:195` — `assert_select ".people-directory__presence", text: "Online", minimum: 1` | rust/crates/campfire/src/controllers/users/people_tests.rs:383 | Actual directory GET; exact active selection/user exclusion, presence/Agent/bar, inactive-row exclusion and minimum rows; ordered/starred complete routed directory DOM and full byte oracle. Anonymous redirect is separately checked. Cases: directories. |
| `test/controllers/users_controller_test.rb:196` — `assert_select ".profile-card__badge", text: "Agent", minimum: 1` | rust/crates/campfire/src/controllers/users/people_tests.rs:384 | Actual directory GET; exact active selection/user exclusion, presence/Agent/bar, inactive-row exclusion and minimum rows; ordered/starred complete routed directory DOM and full byte oracle. Anonymous redirect is separately checked. Cases: directories. |
| `test/controllers/users_controller_test.rb:197` — `assert_select "[data-multi-select-target='bar']", 1` | rust/crates/campfire/src/controllers/users/people_tests.rs:378 | Actual directory GET; exact active selection/user exclusion, presence/Agent/bar, inactive-row exclusion and minimum rows; ordered/starred complete routed directory DOM and full byte oracle. Anonymous redirect is separately checked. Cases: directories. |
| `test/controllers/users_controller_test.rb:200` — `assert_select "input[data-user-id='#{users(:jz).id}']", 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:398 | Real directory GET after JZ deactivation: zero input elements bearing JZ's data-user-id, regardless of type, id, or multi-select attributes. Actual producer control inserts an otherwise-uncredited checkbox and fails this exact count. Cases: directories. |
| `test/controllers/users_controller_test.rb:201` — `assert_select ".people-directory__row", text: /JZ/, count: 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:387 | Actual directory GET; exact active selection/user exclusion, presence/Agent/bar, inactive-row exclusion and minimum rows; ordered/starred complete routed directory DOM and full byte oracle. Anonymous redirect is separately checked. Cases: directories. |

## P0131: index lists starred people first with a star marker

Status: **closed**. Native identities:

- `controllers::users::people_tests::directories_match_complete_rails_body_and_starred_order`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:210` — `assert_response :ok` | rust/crates/campfire/src/controllers/users/people_tests.rs:290 | Actual directory GET; exact active selection/user exclusion, presence/Agent/bar, inactive-row exclusion and minimum rows; ordered/starred complete routed directory DOM and full byte oracle. Anonymous redirect is separately checked. Cases: directories. |
| `test/controllers/users_controller_test.rb:214` — `assert_equal users(:kevin).id, rows.first` | rust/crates/campfire/src/controllers/users/people_tests.rs:277 | Actual directory GET; exact active selection/user exclusion, presence/Agent/bar, inactive-row exclusion and minimum rows; ordered/starred complete routed directory DOM and full byte oracle. Anonymous redirect is separately checked. Cases: directories. |
| `test/controllers/users_controller_test.rb:218` — `assert_equal "★", kevin_row.at_css("[aria-label='Starred by you']").text` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Actual directory GET; exact active selection/user exclusion, presence/Agent/bar, inactive-row exclusion and minimum rows; ordered/starred complete routed directory DOM and full byte oracle. Anonymous redirect is separately checked. Cases: directories. |

## P0132: index requires sign-in

Status: **closed**. Native identities:

- `controllers::users::people_tests::directory_requires_sign_in_and_excludes_the_viewer`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users_controller_test.rb:224` — `assert_redirected_to new_session_url` | rust/crates/campfire/src/controllers/users/people_tests.rs:326 | Actual directory GET; exact active selection/user exclusion, presence/Agent/bar, inactive-row exclusion and minimum rows; ordered/starred complete routed directory DOM and full byte oracle. Anonymous redirect is separately checked. Cases: directories. |

## P0141: update stamps the tour as completed

Status: **closed**. Native identities:

- `controllers::users::preferences_tests::tour_touch_matches_rails_and_refreshes_on_repeated_completion`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/tours_controller_test.rb:10` — `assert_response :no_content` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:184 | Original tour PATCH/read and exact layout/help controls. |
| `test/controllers/users/tours_controller_test.rb:11` — `assert_not_nil users(:david).reload.tour_completed_at` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:200 | Original tour PATCH/read and exact layout/help controls. |

## P0142: update is idempotent

Status: **closed**. Native identities:

- `controllers::users::preferences_tests::tour_touch_matches_rails_and_refreshes_on_repeated_completion`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/tours_controller_test.rb:22` — `assert_response :no_content` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:184 | Original tour PATCH/read and exact layout/help controls. |
| `test/controllers/users/tours_controller_test.rb:23` — `assert_operator users(:david).reload.tour_completed_at, :>, stamped` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:200 | Original tour PATCH/read and exact layout/help controls. |

## P0143: update requires sign-in

Status: **closed**. Native identities:

- `controllers::users::preferences_tests::preference_writes_require_session_and_csrf_and_scope_to_current_user`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/tours_controller_test.rb:30` — `assert_response :redirect` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:89 | Original tour PATCH/read and exact layout/help controls. |

## P0144: room pages carry the tour shell for members who never completed it

Status: **closed**. Native identities:

- `controllers::users::preferences_tests::tour_stamp_controls_the_room_layout_auto_start`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/tours_controller_test.rb:39` — `assert_response :success` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:241 | Original tour PATCH/read and exact layout/help controls. |
| `test/controllers/users/tours_controller_test.rb:40` — `assert_select '#tour[data-controller="tour"][data-tour-auto-start-value="true"]'` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:219 | Original tour PATCH/read and exact layout/help controls. |
| `test/controllers/users/tours_controller_test.rb:41` — `assert_select "#help-menu-button", count: 1` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:224 | Original tour PATCH/read and exact layout/help controls. |

## P0145: room pages skip auto-start once the tour completed

Status: **closed**. Native identities:

- `controllers::users::preferences_tests::tour_stamp_controls_the_room_layout_auto_start`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/tours_controller_test.rb:50` — `assert_response :success` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:257 | Original tour PATCH/read and exact layout/help controls. |
| `test/controllers/users/tours_controller_test.rb:51` — `assert_select '#tour[data-controller="tour"][data-tour-auto-start-value="false"]'` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:219 | Original tour PATCH/read and exact layout/help controls. |
| `test/controllers/users/tours_controller_test.rb:52` — `assert_select "#help-menu-button", count: 1` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:224 | Original tour PATCH/read and exact layout/help controls. |

## P0146: show

Status: **closed**. Native identities:

- `controllers::users::profile_page_tests::profile_route_renders_owner_sections_and_ws9_security_directly`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:13` — `assert_response :success` | rust/crates/campfire/src/controllers/users/profile_page_tests.rs:215 | Real GET succeeds; every owned section and provider connection is mounted. |

## P0147: show gives the Edge install instructions to a browser identifying only as Edge

Status: **closed**. Native identities:

- `controllers::users::profile_page_tests::edge_only_user_agent_gets_rails_edge_install_instructions`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:21` — `assert_response :success` | rust/crates/campfire/src/controllers/users/profile_page_tests.rs:270 | Real GET with the original Edge-only User-Agent; successful response and Edge instructions image/rendered install text. |
| `test/controllers/users/profiles_controller_test.rb:22` — `assert_select "details.pwa__instructions img[src*='install-edge']"` | rust/crates/campfire/src/controllers/users/profile_page_tests.rs:289 | Real GET with the original Edge-only User-Agent; successful response and Edge instructions image/rendered install text. |

## P0148: update

Status: **closed**. Native identities:

- `controllers::users::profile_settings_tests::manual_profile_settings_match_pinned_rails_patch_vectors`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:28` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:263 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_form_profile. |
| `test/controllers/users/profiles_controller_test.rb:29` — `assert_equal "John Doe", users(:david).reload.name` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:281 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_form_profile. |
| `test/controllers/users/profiles_controller_test.rb:30` — `assert_equal "Acrobat", users(:david).bio` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:281 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_form_profile. |
| `test/controllers/users/profiles_controller_test.rb:31` — `assert_equal "david@37signals.com", users(:david).email_address` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:270 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_form_profile. |

## P0149: updates are limited to the current user

Status: **closed**. Native identities:

- `controllers::users::profile_settings_tests::manual_profile_settings_match_pinned_rails_patch_vectors`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:37` — `assert_equal "Jason", users(:jason).reload.name` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:281 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_form_foreign_name. |

## P0150: profile shows Google Calendar as not configured without credentials

Status: **closed**. Native identities:

- `controllers::users::cutover_receipts_tests::original_profile_defaults_and_zone_options_match_rails_through_http`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:45` — `assert_includes response.body, "Google Calendar is not configured for this workspace"` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:80 | Real GET; exact checkbox/selected-option/password-field cardinality and values, notification/meeting explanations, configured/Drive absence. Original nil and Rails/IANA zone options. Cases: pages. |
| `test/controllers/users/profiles_controller_test.rb:46` — `assert_not_includes response.body, "Connect Google Calendar"` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:80 | Real GET; exact checkbox/selected-option/password-field cardinality and values, notification/meeting explanations, configured/Drive absence. Original nil and Rails/IANA zone options. Cases: pages. |

## P0151: profile offers a connect button without an account

Status: **closed**. Native identities:

- `controllers::users::profile_sections_tests::configured_calendar_profile_uses_real_account_metadata_and_forms`
- `controllers::users::profile_sections_tests::missing`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:52` — `assert_includes response.body, "Connect Google Calendar"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:105 | The actual routed connect form label is checked, independently of the form count. Cases: missing. |
| `test/controllers/users/profiles_controller_test.rb:53` — `assert_select "form[action=?][method=post][data-turbo=false]", google_connect_path, count: 1` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:157 | Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: missing. |

## P0152: profile links to connect for meeting status without an account

Status: **closed**. Native identities:

- `controllers::users::profile_sections_tests::configured_calendar_profile_uses_real_account_metadata_and_forms`
- `controllers::users::profile_sections_tests::missing`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:59` — `assert_response :success` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:85 | Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: missing. |
| `test/controllers/users/profiles_controller_test.rb:60` — `assert_select "a[href='#google-calendar-title']", text: "Connect Google Calendar"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:212 | Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: missing. |
| `test/controllers/users/profiles_controller_test.rb:61` — `assert_select "input[name='user[meeting_status_enabled]'][type=checkbox]", count: 0` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:202 | Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: missing. |

## P0153: profile offers the meeting toggle for a connected account

Status: **closed**. Native identities:

- `controllers::users::profile_sections_tests::configured_calendar_profile_uses_real_account_metadata_and_forms`
- `controllers::users::profile_sections_tests::original_calendar`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:69` — `assert_response :success` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:85 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_calendar. |
| `test/controllers/users/profiles_controller_test.rb:70` — `assert_select "input[name='user[meeting_status_enabled]'][type=checkbox]", count: 1` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:202 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_calendar. |
| `test/controllers/users/profiles_controller_test.rb:71` — `assert_includes response.body, "never titles or attendees"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:184 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_calendar. |

## P0154: profile shows the meeting fetch notice when a refresh failed

Status: **closed**. Native identities:

- `controllers::users::profile_sections_tests::profile_failed_calendar_refresh_keeps_the_original_notice`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:82` — `assert_response :success` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:408 | Real GET with the original connected legacy account and Calendar::MeetingRefresh::UNREACHABLE_MESSAGE in its cache; status and the exact Rails-escaped constant in the HTTP body. Cases: fetch_error. |
| `test/controllers/users/profiles_controller_test.rb:83` — `assert_includes response.body, CGI.escapeHTML(Calendar::MeetingRefresh::UNREACHABLE_MESSAGE)` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:431 | Real GET with the original connected legacy account and Calendar::MeetingRefresh::UNREACHABLE_MESSAGE in its cache; status and the exact Rails-escaped constant in the HTTP body. Cases: fetch_error. |

## P0155: profile asks to reconnect for meeting status left on after disconnect

Status: **closed**. Native identities:

- `controllers::users::profile_sections_tests::profile_missing_google_account_keeps_the_meeting_reconnect_link`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:91` — `assert_response :success` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:408 | Real GET with no GoogleAccount or cache, meeting status enabled and calendar OOO disabled. Exact anchor href/text cardinality and status match the pinned Rails selector result; the old retained/rejected-account branch is additional coverage, not evidence for this setup. Cases: missing_account. |
| `test/controllers/users/profiles_controller_test.rb:92` — `assert_select "a[href='#google-calendar-title']", text: "Reconnect below"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:425 | Real GET with no GoogleAccount or cache, meeting status enabled and calendar OOO disabled. Exact anchor href/text cardinality and status match the pinned Rails selector result; the old retained/rejected-account branch is additional coverage, not evidence for this setup. Cases: missing_account. |

## P0156: profile lists the quiet-during-meetings switch

Status: **closed**. Native identities:

- `controllers::users::cutover_receipts_tests::original_profile_defaults_and_zone_options_match_rails_through_http`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:98` — `assert_response :success` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:80 | Real GET; exact checkbox/selected-option/password-field cardinality and values, notification/meeting explanations, configured/Drive absence. Original nil and Rails/IANA zone options. Cases: pages. |
| `test/controllers/users/profiles_controller_test.rb:99` — `assert_select "input[name='user[meeting_dnd_enabled]'][type=checkbox]", count: 1` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:80 | Real GET; exact checkbox/selected-option/password-field cardinality and values, notification/meeting explanations, configured/Drive absence. Original nil and Rails/IANA zone options. Cases: pages. |
| `test/controllers/users/profiles_controller_test.rb:100` — `assert_includes response.body, "Do not disturb during meetings"` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:80 | Real GET; exact checkbox/selected-option/password-field cardinality and values, notification/meeting explanations, configured/Drive absence. Original nil and Rails/IANA zone options. Cases: pages. |

## P0157: the layout sends meeting windows for the live sound gate

Status: **closed**. Native identities:

- `controllers::users::layout_preferences_tests::layout_meeting_windows_include_all_cached_pairs`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:112` — `assert_response :success` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:57 | Real GET; entire sound, Drive and zone meta lines match pinned Rails bytes, plus exact html theme/text-size attributes. Cache/user inputs match each original active/future/off scenario. Cases: meeting_current. |
| `test/controllers/users/profiles_controller_test.rb:113` — `assert_select "meta[name=notification-dnd]", count: 0` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Cases: meeting_current. |
| `test/controllers/users/profiles_controller_test.rb:114` — `assert_select "meta[name=meeting-quiet][content=?]", "#{start_at.to_i}-#{end_at.to_i}", count: 1` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Cases: meeting_current. |

## P0158: the layout sends future meeting windows before the meeting starts

Status: **closed**. Native identities:

- `controllers::users::layout_preferences_tests::layout_future_meeting_windows_before_start`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:124` — `assert_response :success` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:57 | Real GET; entire sound, Drive and zone meta lines match pinned Rails bytes, plus exact html theme/text-size attributes. Cache/user inputs match each original active/future/off scenario. Cases: meeting_future. |
| `test/controllers/users/profiles_controller_test.rb:125` — `assert_select "meta[name=meeting-quiet]", count: 1` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Cases: meeting_future. |

## P0159: the layout sends no meeting windows without cached intervals

Status: **closed**. Native identities:

- `controllers::users::layout_preferences_tests::layout_empty_or_missing_meeting_cache`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:134` — `assert_response :success` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:57 | Real GET; entire sound, Drive and zone meta lines match pinned Rails bytes, plus exact html theme/text-size attributes. Cache/user inputs match each original active/future/off scenario. Cases: meeting_empty, meeting_missing. |
| `test/controllers/users/profiles_controller_test.rb:135` — `assert_select "meta[name=meeting-quiet]", count: 0` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Cases: meeting_empty, meeting_missing. |

## P0160: the layout sends no meeting windows when meeting status itself is off

Status: **closed**. Native identities:

- `controllers::users::layout_preferences_tests::layout_meeting_status_off_sends_no_windows`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:145` — `assert_response :success` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:57 | Real GET; entire sound, Drive and zone meta lines match pinned Rails bytes, plus exact html theme/text-size attributes. Cache/user inputs match each original active/future/off scenario. Cases: meeting_status_off. |
| `test/controllers/users/profiles_controller_test.rb:146` — `assert_select "meta[name=meeting-quiet]", count: 0` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Cases: meeting_status_off. |

## P0161: the layout sends OOO windows for the live sound gate

Status: **closed**. Native identities:

- `controllers::users::layout_preferences_tests::layout_manual_ooo_windows`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:154` — `assert_response :success` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:57 | Real GET; entire sound, Drive and zone meta lines match pinned Rails bytes, plus exact html theme/text-size attributes. Cache/user inputs match each original active/future/off scenario. Cases: manual_ooo. |
| `test/controllers/users/profiles_controller_test.rb:155` — `assert_select "meta[name=ooo-quiet][content=?]", "0-#{users(:david).ooo_until.to_i}", count: 1` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Cases: manual_ooo. |

## P0162: the layout sends future calendar OOO windows before the OOO starts

Status: **closed**. Native identities:

- `controllers::users::layout_preferences_tests::layout_future_calendar_ooo_windows`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:165` — `assert_response :success` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:57 | Real GET; entire sound, Drive and zone meta lines match pinned Rails bytes, plus exact html theme/text-size attributes. Cache/user inputs match each original active/future/off scenario. Cases: calendar_ooo_future. |
| `test/controllers/users/profiles_controller_test.rb:166` — `assert_select "meta[name=ooo-quiet]", count: 1` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Cases: calendar_ooo_future. |

## P0163: the layout sends no OOO windows when keeping notifications while out

Status: **closed**. Native identities:

- `controllers::users::layout_preferences_tests::layout_ooo_notifications_kept_sends_no_windows`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:174` — `assert_response :success` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:57 | Real GET; entire sound, Drive and zone meta lines match pinned Rails bytes, plus exact html theme/text-size attributes. Cache/user inputs match each original active/future/off scenario. Original setup retained: manual-only OOO/notifications kept, or quiet-hours disabled while stored zone remains UTC. Cases: ooo_notifications_kept, original_manual_ooo_notifications_kept. |
| `test/controllers/users/profiles_controller_test.rb:175` — `assert_select "meta[name=ooo-quiet]", count: 0` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Original setup retained: manual-only OOO/notifications kept, or quiet-hours disabled while stored zone remains UTC. Cases: ooo_notifications_kept, original_manual_ooo_notifications_kept. |

## P0164: the layout leaves sounds alone for meetings when quiet-during-meetings is off

Status: **closed**. Native identities:

- `controllers::users::layout_preferences_tests::layout_meeting_quiet_off_sends_no_windows`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:185` — `assert_response :success` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:57 | Real GET; entire sound, Drive and zone meta lines match pinned Rails bytes, plus exact html theme/text-size attributes. Cache/user inputs match each original active/future/off scenario. Cases: meeting_quiet_off. |
| `test/controllers/users/profiles_controller_test.rb:186` — `assert_select "meta[name=notification-dnd]", count: 0` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Cases: meeting_quiet_off. |
| `test/controllers/users/profiles_controller_test.rb:187` — `assert_select "meta[name=meeting-quiet]", count: 0` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Cases: meeting_quiet_off. |

## P0165: profile shows the connected account with a disconnect button

Status: **closed**. Native identities:

- `controllers::users::profile_sections_tests::configured_calendar_profile_uses_real_account_metadata_and_forms`
- `controllers::users::profile_sections_tests::original_calendar`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:195` — `assert_includes response.body, "Connected as david@gmail.test"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:113 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real original legacy account david@gmail.test; connected label and Disconnect in the complete HTTP section, compared with the pinned Rails bytes. Cases: original_calendar. |
| `test/controllers/users/profiles_controller_test.rb:196` — `assert_includes response.body, "Disconnect"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:122 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real original legacy account david@gmail.test; connected label and Disconnect in the complete HTTP section, compared with the pinned Rails bytes. Cases: original_calendar. |

## P0166: profile offers a reconnect when Google rejected the connection

Status: **closed**. Native identities:

- `controllers::users::profile_sections_tests::configured_calendar_profile_uses_real_account_metadata_and_forms`
- `controllers::users::profile_sections_tests::original_rejected_calendar`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:204` — `assert_includes response.body, "Google rejected the connection, reconnect"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:143 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_rejected_calendar. |
| `test/controllers/users/profiles_controller_test.rb:205` — `assert_includes response.body, "Connect Google Calendar"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:105 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. The actual routed connect form label is checked, independently of the form count. Cases: original_rejected_calendar. |
| `test/controllers/users/profiles_controller_test.rb:206` — `assert_select "form[action=?][method=post][data-turbo=false]", google_connect_path, count: 1` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:157 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_rejected_calendar. |

## P0167: profile offers Drive previews for a connected account without the Drive scope

Status: **closed**. Native identities:

- `controllers::users::profile_sections_tests::configured_calendar_profile_uses_real_account_metadata_and_forms`
- `controllers::users::profile_sections_tests::original_calendar`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:214` — `assert_includes response.body, "Enable Drive previews"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:132 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_calendar. |
| `test/controllers/users/profiles_controller_test.rb:215` — `assert_not_includes response.body, "Drive previews enabled"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:128 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_calendar. |
| `test/controllers/users/profiles_controller_test.rb:216` — `assert_select "form[action=?][method=post][data-turbo=false]", google_connect_path, count: 1 do` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:157 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_calendar. |
| `test/controllers/users/profiles_controller_test.rb:217` — `assert_select "input[name='features[]'][value=drive]", count: 1` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:173 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_calendar. |

## P0168: profile shows Drive previews as enabled when the account has the Drive scope

Status: **closed**. Native identities:

- `controllers::users::profile_sections_tests::configured_calendar_profile_uses_real_account_metadata_and_forms`
- `controllers::users::profile_sections_tests::original_calendar_drive`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:226` — `assert_includes response.body, "Drive previews enabled"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:128 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_calendar_drive. |
| `test/controllers/users/profiles_controller_test.rb:227` — `assert_not_includes response.body, "Enable Drive previews"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:132 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_calendar_drive. |

## P0169: profile offers Drive previews again for the retired metadata grant

Status: **closed**. Native identities:

- `controllers::users::profile_sections_tests::configured_calendar_profile_uses_real_account_metadata_and_forms`
- `controllers::users::profile_sections_tests::original_retired_metadata`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:235` — `assert_includes response.body, "Enable Drive previews"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:132 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_retired_metadata. |
| `test/controllers/users/profiles_controller_test.rb:236` — `assert_not_includes response.body, "Drive previews enabled"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:128 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_retired_metadata. |

## P0170: profile shows no Drive row when Google is not configured

Status: **closed**. Native identities:

- `controllers::users::cutover_receipts_tests::original_profile_defaults_and_zone_options_match_rails_through_http`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:244` — `assert_not_includes response.body, "Enable Drive previews"` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:80 | Real GET; exact checkbox/selected-option/password-field cardinality and values, notification/meeting explanations, configured/Drive absence. Original nil and Rails/IANA zone options. Cases: pages. |
| `test/controllers/users/profiles_controller_test.rb:245` — `assert_not_includes response.body, "Drive previews enabled"` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:80 | Real GET; exact checkbox/selected-option/password-field cardinality and values, notification/meeting explanations, configured/Drive absence. Original nil and Rails/IANA zone options. Cases: pages. |

## P0171: profile asks to reconnect when the grant lacks the calendar scope

Status: **closed**. Native identities:

- `controllers::users::profile_sections_tests::profile_asks_to_reconnect_for_an_openid_email_only_grant`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:253` — `assert_includes response.body, "Calendar permission needed, reconnect to publish events"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:352 | Real profile GET with configured OAuth and a persisted david@gmail.test account whose scopes are exactly openid email (no Calendar, no Drive). Whole-body warning/Connected-as absence/Disconnect and one form with action, method and data-turbo on the same element; complete Rails fragment bytes are also compared by calendar_case. Cases: openid_email. |
| `test/controllers/users/profiles_controller_test.rb:254` — `assert_not_includes response.body, "Connected as"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:353 | Real profile GET with configured OAuth and a persisted david@gmail.test account whose scopes are exactly openid email (no Calendar, no Drive). Whole-body warning/Connected-as absence/Disconnect and one form with action, method and data-turbo on the same element; complete Rails fragment bytes are also compared by calendar_case. Cases: openid_email. |
| `test/controllers/users/profiles_controller_test.rb:255` — `assert_includes response.body, "Disconnect"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:354 | Real profile GET with configured OAuth and a persisted david@gmail.test account whose scopes are exactly openid email (no Calendar, no Drive). Whole-body warning/Connected-as absence/Disconnect and one form with action, method and data-turbo on the same element; complete Rails fragment bytes are also compared by calendar_case. Cases: openid_email. |
| `test/controllers/users/profiles_controller_test.rb:256` — `assert_select "form[action=?][method=post][data-turbo=false]", google_connect_path, count: 1` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:367 | Real profile GET with configured OAuth and a persisted david@gmail.test account whose scopes are exactly openid email (no Calendar, no Drive). Whole-body warning/Connected-as absence/Disconnect and one form with action, method and data-turbo on the same element; complete Rails fragment bytes are also compared by calendar_case. Cases: openid_email. |

## P0172: profile shows Disconnect for a partial grant with Drive still active

Status: **closed**. Native identities:

- `controllers::users::profile_sections_tests::configured_calendar_profile_uses_real_account_metadata_and_forms`
- `controllers::users::profile_sections_tests::original_drive_only`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:265` — `assert_includes response.body, "Calendar permission needed, reconnect to publish events"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:145 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_drive_only. |
| `test/controllers/users/profiles_controller_test.rb:266` — `assert_includes response.body, "Disconnect"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:122 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_drive_only. |
| `test/controllers/users/profiles_controller_test.rb:267` — `assert_includes response.body, "Connect Google Calendar"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:105 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. The actual routed connect form label is checked, independently of the form count. Cases: original_drive_only. |

## P0173: reconnect preserves a granted Drive scope

Status: **closed**. Native identities:

- `controllers::users::profile_sections_tests::configured_calendar_profile_uses_real_account_metadata_and_forms`
- `controllers::users::profile_sections_tests::original_rejected_calendar_drive`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:275` — `assert_includes response.body, "Google rejected the connection, reconnect"` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:143 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_rejected_calendar_drive. |
| `test/controllers/users/profiles_controller_test.rb:276` — `assert_select "form[action=?][method=post][data-turbo=false]", google_connect_path, count: 1 do` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:157 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_rejected_calendar_drive. |
| `test/controllers/users/profiles_controller_test.rb:277` — `assert_select "input[name='features[]'][value=drive]", count: 1` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:173 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_rejected_calendar_drive. |

## P0174: reconnect without Drive requests the calendar scope only

Status: **closed**. Native identities:

- `controllers::users::profile_sections_tests::configured_calendar_profile_uses_real_account_metadata_and_forms`
- `controllers::users::profile_sections_tests::original_rejected_calendar`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:286` — `assert_select "form[action=?][method=post][data-turbo=false]", google_connect_path, count: 1 do` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:157 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_rejected_calendar. |
| `test/controllers/users/profiles_controller_test.rb:287` — `assert_select "input[name='features[]']", count: 0` | rust/crates/campfire/src/controllers/users/profile_sections_tests.rs:173 | Real GET with the exact original Google helper state/scopes/email/rejection, not just equivalent capability booleans. Complete HTTP section bytes, connect-form ancestry/method/data-turbo, Drive feature count/value, warning and Disconnect/meeting controls. Real GET with workspace configuration and persisted Google account/scopes/rejection. Exact connect-form count/method/data-turbo, Drive hidden feature count/value, status, meeting toggle, explanation and connect/disconnect/Drive labels. Complete fragment oracle separately preserves byte parity. Cases: original_rejected_calendar. |

## P0175: layout carries the Drive previews meta tag only with the Drive scope

Status: **closed**. Native identities:

- `controllers::users::layout_preferences_tests::layout_drive_previews_uses_exact_scope`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:293` — `assert_not_includes response.body, "google-drive-previews"` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Original sequence: absent account, legacy Calendar account, then exact openid/email/Calendar/Drive grant. Cases: drive_missing, original_drive_legacy_calendar, original_drive_full_scope. |
| `test/controllers/users/profiles_controller_test.rb:297` — `assert_not_includes response.body, "google-drive-previews"` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Original sequence: absent account, legacy Calendar account, then exact openid/email/Calendar/Drive grant. Cases: drive_missing, original_drive_legacy_calendar, original_drive_full_scope. |
| `test/controllers/users/profiles_controller_test.rb:301` — `assert_includes response.body, '<meta name="google-drive-previews" content="enabled">'` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Original sequence: absent account, legacy Calendar account, then exact openid/email/Calendar/Drive grant. Cases: drive_missing, original_drive_legacy_calendar, original_drive_full_scope. |

## P0176: linking a github login strips and downcases it

Status: **closed**. Native identities:

- `controllers::users::profile_settings_tests::manual_profile_settings_match_pinned_rails_patch_vectors`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:307` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:263 | Real original form-encoded PUT and PATCH with CSRF; exact redirect and persisted state equal independently recorded Rails. Clear starts with david-gh; linking uses the original spaced David-GH input. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_form_github. |
| `test/controllers/users/profiles_controller_test.rb:308` — `assert_equal "david-gh", users(:david).reload.github_login` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:281 | Real original form-encoded PUT and PATCH with CSRF; exact redirect and persisted state equal independently recorded Rails. Clear starts with david-gh; linking uses the original spaced David-GH input. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_form_github. |

## P0177: a github login cannot be claimed by a second user

Status: **closed**. Native identities:

- `controllers::users::cutover_receipts_tests::original_profile_duplicate_login_is_rejected`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:316` — `assert_response :unprocessable_entity` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:113 | Real PUT with original Rails values; exact status, location, text_size/voice_mode/raw preferences/login state and duplicate error. The raw true/false strings are preserved and typed preference semantics are checked below. Cases: mutations. |
| `test/controllers/users/profiles_controller_test.rb:317` — `assert_select "p", text: /already linked to another user/` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:139 | Real PUT with original Rails values; exact status, location, text_size/voice_mode/raw preferences/login state and duplicate error. The raw true/false strings are preserved and typed preference semantics are checked below. Cases: mutations. |
| `test/controllers/users/profiles_controller_test.rb:318` — `assert_nil users(:david).reload.github_login` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:130 | Real PUT with original Rails values; exact status, location, text_size/voice_mode/raw preferences/login state and duplicate error. The raw true/false strings are preserved and typed preference semantics are checked below. Cases: mutations. |

## P0178: profile lists the notification switches with explanations

Status: **closed**. Native identities:

- `controllers::users::cutover_receipts_tests::original_profile_defaults_and_zone_options_match_rails_through_http`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:324` — `assert_response :success` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:80 | Real GET; exact checkbox/selected-option/password-field cardinality and values, notification/meeting explanations, configured/Drive absence. Original nil and Rails/IANA zone options. Cases: pages. |
| `test/controllers/users/profiles_controller_test.rb:326` — `assert_select "input[name='user[inbox_preferences][#{key}]'][type=checkbox][checked]"` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:80 | Real GET; exact checkbox/selected-option/password-field cardinality and values, notification/meeting explanations, configured/Drive absence. Original nil and Rails/IANA zone options. Cases: pages. |
| `test/controllers/users/profiles_controller_test.rb:328` — `assert_includes response.body, "GitHub review requests"` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:80 | Real GET; exact checkbox/selected-option/password-field cardinality and values, notification/meeting explanations, configured/Drive absence. Original nil and Rails/IANA zone options. Cases: pages. |
| `test/controllers/users/profiles_controller_test.rb:329` — `assert_includes response.body, "The incoming-call banner still shows."` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:80 | Real GET; exact checkbox/selected-option/password-field cardinality and values, notification/meeting explanations, configured/Drive absence. Original nil and Rails/IANA zone options. Cases: pages. |

## P0179: profile saves the notification switches

Status: **closed**. Native identities:

- `controllers::users::cutover_receipts_tests::original_profile_notification_switch_save_matches_rails`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:341` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:119 | Real PUT using original values; exact status/location and saved settings, plus a real GET compared with Rails typed preferences. Cases: original_notifications. |
| `test/controllers/users/profiles_controller_test.rb:343` — `assert_equal false, preferences.github_review_requests` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:149 | After the real PUT, the real GET checked checkbox names match Rails typed boolean preferences for all five keys; raw stored JSON alone is not sufficient. Cases: original_notifications. |
| `test/controllers/users/profiles_controller_test.rb:344` — `assert_equal false, preferences.agent_approvals` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:149 | After the real PUT, the real GET checked checkbox names match Rails typed boolean preferences for all five keys; raw stored JSON alone is not sufficient. Cases: original_notifications. |
| `test/controllers/users/profiles_controller_test.rb:345` — `assert_equal true, preferences.agent_work` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:149 | After the real PUT, the real GET checked checkbox names match Rails typed boolean preferences for all five keys; raw stored JSON alone is not sufficient. Cases: original_notifications. |
| `test/controllers/users/profiles_controller_test.rb:346` — `assert_equal false, preferences.event_reminders` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:149 | After the real PUT, the real GET checked checkbox names match Rails typed boolean preferences for all five keys; raw stored JSON alone is not sufficient. Cases: original_notifications. |
| `test/controllers/users/profiles_controller_test.rb:347` — `assert_equal true, preferences.huddle_invitations` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:149 | After the real PUT, the real GET checked checkbox names match Rails typed boolean preferences for all five keys; raw stored JSON alone is not sufficient. Cases: original_notifications. |

## P0180: profile rejects non-boolean notification input

Status: **closed**. Native identities:

- `controllers::users::cutover_receipts_tests::original_profile_bad_notification_preserves_saved_state`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:353` — `assert_response :unprocessable_entity` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:113 | Real PUT using original values; exact status/location and saved settings, plus a real GET compared with Rails typed preferences. Cases: original_bad_notification. |
| `test/controllers/users/profiles_controller_test.rb:354` — `assert_equal true, users(:david).reload.inbox_preferences.github_review_requests` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:149 | Real PUT using original values; exact status/location and saved settings, plus a real GET compared with Rails typed preferences. Cases: original_bad_notification. |

## P0181: DND switch reflects the effective state after a timed expiry

Status: **closed**. Native identities:

- `controllers::users::profile_page_tests::dnd_switch_tracks_expired_and_live_timers`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:362` — `assert_response :success` | rust/crates/campfire/src/controllers/users/profile_page_tests.rs:310 | Real GET with expired/live dnd_until; checkbox checked attribute follows effective state. |
| `test/controllers/users/profiles_controller_test.rb:363` — `assert_select "#user_dnd_enabled[checked]", count: 0` | rust/crates/campfire/src/controllers/users/profile_page_tests.rs:330 | Real GET with expired/live dnd_until; checkbox checked attribute follows effective state. |

## P0182: DND switch stays on while a timer runs

Status: **closed**. Native identities:

- `controllers::users::profile_page_tests::dnd_switch_tracks_expired_and_live_timers`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:371` — `assert_response :success` | rust/crates/campfire/src/controllers/users/profile_page_tests.rs:310 | Real GET with expired/live dnd_until; checkbox checked attribute follows effective state. |
| `test/controllers/users/profiles_controller_test.rb:372` — `assert_select "#user_dnd_enabled[checked]", count: 1` | rust/crates/campfire/src/controllers/users/profile_page_tests.rs:330 | Real GET with expired/live dnd_until; checkbox checked attribute follows effective state. |

## P0183: profile lists the call settings with their defaults

Status: **closed**. Native identities:

- `controllers::users::cutover_receipts_tests::original_profile_defaults_and_zone_options_match_rails_through_http`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:378` — `assert_response :success` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:80 | Real GET; exact checkbox/selected-option/password-field cardinality and values, notification/meeting explanations, configured/Drive absence. Original nil and Rails/IANA zone options. Cases: pages. |
| `test/controllers/users/profiles_controller_test.rb:379` — `assert_select "select[name='user[voice_mode]'] option[selected]", text: "Voice activity"` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:80 | Real GET; exact checkbox/selected-option/password-field cardinality and values, notification/meeting explanations, configured/Drive absence. Original nil and Rails/IANA zone options. Cases: pages. |
| `test/controllers/users/profiles_controller_test.rb:380` — `assert_select "input[name='user[push_to_talk_key]'][value='&#96;']"` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:80 | Real GET; exact checkbox/selected-option/password-field cardinality and values, notification/meeting explanations, configured/Drive absence. Original nil and Rails/IANA zone options. Cases: pages. |

## P0184: profile saves the call settings

Status: **closed**. Native identities:

- `controllers::users::profile_settings_tests::manual_profile_settings_match_pinned_rails_patch_vectors`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:386` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:263 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_form_voice. |
| `test/controllers/users/profiles_controller_test.rb:388` — `assert_equal "push_to_talk", user.voice_mode` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:281 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_form_voice. |
| `test/controllers/users/profiles_controller_test.rb:389` — `assert_equal "CapsLock", user.push_to_talk_key` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:281 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_form_voice. |

## P0185: profile rejects an unknown microphone mode

Status: **closed**. Native identities:

- `controllers::users::cutover_receipts_tests::original_profile_bad_voice_preserves_saved_state`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:395` — `assert_response :unprocessable_entity` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:113 | Real PUT using original values; exact status/location and saved settings, plus a real GET compared with Rails typed preferences. Cases: original_bad_voice. |
| `test/controllers/users/profiles_controller_test.rb:396` — `assert_equal "voice_activity", users(:david).reload.voice_mode` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:130 | Real PUT using original values; exact status/location and saved settings, plus a real GET compared with Rails typed preferences. Cases: original_bad_voice. |

## P0186: clearing a github login unlinks it

Status: **closed**. Native identities:

- `controllers::users::profile_settings_tests::manual_profile_settings_match_pinned_rails_patch_vectors`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:404` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:263 | Real original form-encoded PUT and PATCH with CSRF; exact redirect and persisted state equal independently recorded Rails. Clear starts with david-gh; linking uses the original spaced David-GH input. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_form_github_clear. |
| `test/controllers/users/profiles_controller_test.rb:405` — `assert_nil users(:david).reload.github_login` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:281 | Real original form-encoded PUT and PATCH with CSRF; exact redirect and persisted state equal independently recorded Rails. Clear starts with david-gh; linking uses the original spaced David-GH input. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_form_github_clear. |

## P0187: changing email requires the current password

Status: **closed**. Native identities:

- `app::profile_security_tests::profile_guard_fields_errors_and_security_writes_match_pinned_rails`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:411` — `assert_response :unprocessable_entity` | rust/crates/campfire/src/app/profile_security_tests.rs:267 | Real GET/PATCH with existing password digest; exact status, saved name/bio/email, password preservation, self-change timestamp, audits and device revocation; original error/input fragment and passwordless field absence. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_missing. |
| `test/controllers/users/profiles_controller_test.rb:412` — `assert_includes response.body, "Current password is required to change your email address"` | rust/crates/campfire/src/app/profile_security_tests.rs:331 | Real GET/PATCH with existing password digest; exact status, saved name/bio/email, password preservation, self-change timestamp, audits and device revocation; original error/input fragment and passwordless field absence. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_missing. |
| `test/controllers/users/profiles_controller_test.rb:413` — `assert_equal "david@37signals.com", users(:david).reload.email_address` | rust/crates/campfire/src/app/profile_security_tests.rs:286 | Real GET/PATCH with existing password digest; exact status, saved name/bio/email, password preservation, self-change timestamp, audits and device revocation; original error/input fragment and passwordless field absence. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_missing. |
| `test/controllers/users/profiles_controller_test.rb:414` — `assert_nil users(:david).email_self_changed_at` | rust/crates/campfire/src/app/profile_security_tests.rs:306 | Real GET/PATCH with existing password digest; exact status, saved name/bio/email, password preservation, self-change timestamp, audits and device revocation; original error/input fragment and passwordless field absence. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_missing. |

## P0188: changing email with a wrong current password is refused

Status: **closed**. Native identities:

- `app::profile_security_tests::profile_guard_fields_errors_and_security_writes_match_pinned_rails`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:420` — `assert_response :unprocessable_entity` | rust/crates/campfire/src/app/profile_security_tests.rs:267 | Real GET/PATCH with existing password digest; exact status, saved name/bio/email, password preservation, self-change timestamp, audits and device revocation; original error/input fragment and passwordless field absence. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_wrong. |
| `test/controllers/users/profiles_controller_test.rb:421` — `assert_includes response.body, "Current password is incorrect"` | rust/crates/campfire/src/app/profile_security_tests.rs:331 | Real GET/PATCH with existing password digest; exact status, saved name/bio/email, password preservation, self-change timestamp, audits and device revocation; original error/input fragment and passwordless field absence. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_wrong. |
| `test/controllers/users/profiles_controller_test.rb:422` — `assert_equal "david@37signals.com", users(:david).reload.email_address` | rust/crates/campfire/src/app/profile_security_tests.rs:286 | Real GET/PATCH with existing password digest; exact status, saved name/bio/email, password preservation, self-change timestamp, audits and device revocation; original error/input fragment and passwordless field absence. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_wrong. |

## P0189: a new password cannot stand in for the current one

Status: **closed**. Native identities:

- `app::profile_security_tests::profile_guard_fields_errors_and_security_writes_match_pinned_rails`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:428` — `assert_response :unprocessable_entity` | rust/crates/campfire/src/app/profile_security_tests.rs:267 | Real GET/PATCH with existing password digest; exact status, saved name/bio/email, password preservation, self-change timestamp, audits and device revocation; original error/input fragment and passwordless field absence. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_new_is_not_current. |
| `test/controllers/users/profiles_controller_test.rb:429` — `assert_equal "david@37signals.com", users(:david).reload.email_address` | rust/crates/campfire/src/app/profile_security_tests.rs:286 | Real GET/PATCH with existing password digest; exact status, saved name/bio/email, password preservation, self-change timestamp, audits and device revocation; original error/input fragment and passwordless field absence. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_new_is_not_current. |
| `test/controllers/users/profiles_controller_test.rb:430` — `assert users(:david).authenticate("secret123456")` | rust/crates/campfire/src/app/profile_security_tests.rs:282 | The reloaded persisted user authenticates with the original password after the real refused write, rather than merely comparing digests. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_new_is_not_current. |

## P0190: changing email with the current password records a self-change

Status: **closed**. Native identities:

- `app::profile_security_tests::profile_guard_fields_errors_and_security_writes_match_pinned_rails`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:437` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/app/profile_security_tests.rs:274 | Original form-encoded PUT and PATCH: exact redirect, persisted name/email and self-change timestamp equal Rails; email audits, error HTML, password preservation and device effects are checked on the same request. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_correct. |
| `test/controllers/users/profiles_controller_test.rb:438` — `assert_equal "david@smartdata.net", users(:david).reload.email_address` | rust/crates/campfire/src/app/profile_security_tests.rs:286 | Original form-encoded PUT and PATCH: exact redirect, persisted name/email and self-change timestamp equal Rails; email audits, error HTML, password preservation and device effects are checked on the same request. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_correct. |
| `test/controllers/users/profiles_controller_test.rb:439` — `assert_equal Time.current, users(:david).email_self_changed_at` | rust/crates/campfire/src/app/profile_security_tests.rs:306 | Original form-encoded PUT and PATCH: exact redirect, persisted name/email and self-change timestamp equal Rails; email audits, error HTML, password preservation and device effects are checked on the same request. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_correct. |

## P0191: other profile edits and case-only email edits need no password and record nothing

Status: **closed**. Native identities:

- `app::profile_security_tests::profile_guard_fields_errors_and_security_writes_match_pinned_rails`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:446` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/app/profile_security_tests.rs:274 | Original form-encoded PUT and PATCH: exact redirect, persisted name/email and self-change timestamp equal Rails; email audits, error HTML, password preservation and device effects are checked on the same request. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_case_only. |
| `test/controllers/users/profiles_controller_test.rb:447` — `assert_equal "Dave", users(:david).reload.name` | rust/crates/campfire/src/app/profile_security_tests.rs:287 | Original form-encoded PUT and PATCH: exact redirect, persisted name/email and self-change timestamp equal Rails; email audits, error HTML, password preservation and device effects are checked on the same request. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_case_only. |
| `test/controllers/users/profiles_controller_test.rb:448` — `assert_nil users(:david).email_self_changed_at` | rust/crates/campfire/src/app/profile_security_tests.rs:306 | Original form-encoded PUT and PATCH: exact redirect, persisted name/email and self-change timestamp equal Rails; email audits, error HTML, password preservation and device effects are checked on the same request. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_case_only. |

## P0192: profile asks for the current password only when the account has one

Status: **closed**. Native identities:

- `app::profile_security_tests::profile_guard_fields_errors_and_security_writes_match_pinned_rails`
- `controllers::users::cutover_receipts_tests::original_profile_defaults_and_zone_options_match_rails_through_http`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:453` — `assert_select "input[name=?][autocomplete=current-password]", "user[current_password]"` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:80 | Real GET/PATCH with existing password digest; exact status, saved name/bio/email, password preservation, self-change timestamp, audits and device revocation; original error/input fragment and passwordless field absence. Cases: passwordless, missing. |
| `test/controllers/users/profiles_controller_test.rb:457` — `assert_select "input[name=?]", "user[current_password]", count: 0` | rust/crates/campfire/src/app/profile_security_tests.rs:236 | Actual input[name=user[current_password]] count is zero on the passwordless profile. Cases: passwordless, missing. |

## P0193: update saves the theme and time zone

Status: **closed**. Native identities:

- `controllers::users::profile_settings_tests::manual_profile_settings_match_pinned_rails_patch_vectors`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:463` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:263 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_form_theme_zone. |
| `test/controllers/users/profiles_controller_test.rb:464` — `assert_equal "dark", users(:david).reload.theme` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:281 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_form_theme_zone. |
| `test/controllers/users/profiles_controller_test.rb:465` — `assert_equal "Pacific Time (US & Canada)", users(:david).time_zone` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:281 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: original_form_theme_zone. |

## P0194: update saves the text size

Status: **closed**. Native identities:

- `controllers::users::cutover_receipts_tests::original_profile_text_size_save_matches_rails`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:471` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:119 | Real PUT using original values; exact status/location and saved settings, plus a real GET compared with Rails typed preferences. Cases: original_text_size. |
| `test/controllers/users/profiles_controller_test.rb:472` — `assert_equal "smaller", users(:david).reload.text_size` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:130 | Real PUT using original values; exact status/location and saved settings, plus a real GET compared with Rails typed preferences. Cases: original_text_size. |

## P0195: an IANA time zone round-trips through the form

Status: **closed**. Native identities:

- `controllers::users::profile_settings_tests::manual_timezone_choice_is_rendered_and_blocks_browser_detection`
- `controllers::users::cutover_receipts_tests::original_profile_defaults_and_zone_options_match_rails_through_http`
- `controllers::users::profile_settings_tests::manual_profile_settings_match_pinned_rails_patch_vectors`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:479` — `assert_response :success` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:74 | Real PATCH then GET then automatic detection; selected IANA option, saved zone, explicit Not set meta and detection refusal. Original selected IANA pre-save GET is also observed in original_profile_defaults_and_zone_options_match_rails_through_http. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: pages[America/New_York]. |
| `test/controllers/users/profiles_controller_test.rb:480` — `assert_select "select#user_time_zone option[selected][value='America/New_York']"` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:80 | Real PATCH then GET then automatic detection; selected IANA option, saved zone, explicit Not set meta and detection refusal. Original selected IANA pre-save GET is also observed in original_profile_defaults_and_zone_options_match_rails_through_http. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: pages[America/New_York]. |
| `test/controllers/users/profiles_controller_test.rb:483` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:263 | Real PATCH then GET then automatic detection; selected IANA option, saved zone, explicit Not set meta and detection refusal. Original selected IANA pre-save GET is also observed in original_profile_defaults_and_zone_options_match_rails_through_http. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: pages[America/New_York]. |
| `test/controllers/users/profiles_controller_test.rb:484` — `assert_equal "America/New_York", users(:david).reload.time_zone` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:94 | Real PATCH then GET then automatic detection; selected IANA option, saved zone, explicit Not set meta and detection refusal. Original selected IANA pre-save GET is also observed in original_profile_defaults_and_zone_options_match_rails_through_http. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: pages[America/New_York]. |
| `test/controllers/users/profiles_controller_test.rb:487` — `assert_select "select#user_time_zone option[selected][value='America/New_York']"` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:80 | Real PATCH then GET then automatic detection; selected IANA option, saved zone, explicit Not set meta and detection refusal. Original selected IANA pre-save GET is also observed in original_profile_defaults_and_zone_options_match_rails_through_http. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: pages[America/New_York]. |

## P0196: a legacy Rails time zone name still shows selected

Status: **closed**. Native identities:

- `controllers::users::cutover_receipts_tests::original_profile_defaults_and_zone_options_match_rails_through_http`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:494` — `assert_response :success` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:80 | Real GET; exact checkbox/selected-option/password-field cardinality and values, notification/meeting explanations, configured/Drive absence. Original nil and Rails/IANA zone options. Cases: pages. |
| `test/controllers/users/profiles_controller_test.rb:495` — `assert_select "select#user_time_zone option[selected][value='America/Los_Angeles']"` | rust/crates/campfire/src/controllers/users/cutover_receipts_tests.rs:80 | Real GET; exact checkbox/selected-option/password-field cardinality and values, notification/meeting explanations, configured/Drive absence. Original nil and Rails/IANA zone options. Cases: pages. |

## P0197: choosing a time zone or Not set records an explicit choice

Status: **closed**. Native identities:

- `controllers::users::profile_settings_tests::manual_profile_settings_match_pinned_rails_patch_vectors`
- `controllers::users::profile_settings_tests::manual_timezone_choice_is_rendered_and_blocks_browser_detection`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:500` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:263 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: appearance, not_set. |
| `test/controllers/users/profiles_controller_test.rb:501` — `assert_equal "America/New_York", users(:david).reload.time_zone` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:281 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: appearance, not_set. |
| `test/controllers/users/profiles_controller_test.rb:502` — `assert users(:david).time_zone_explicit?` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:281 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: appearance, not_set. |
| `test/controllers/users/profiles_controller_test.rb:505` — `assert_redirected_to user_profile_url` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:263 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: appearance, not_set. |
| `test/controllers/users/profiles_controller_test.rb:506` — `assert_nil users(:david).reload.time_zone` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:281; rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:110 | Real PATCH vectors start not_set with America/New_York already saved; full persisted state must clear it and retain the explicit flag. Additional real select/detect/clear sequence checks (None, true) directly. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: appearance, not_set. |
| `test/controllers/users/profiles_controller_test.rb:507` — `assert users(:david).time_zone_explicit?` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:281; rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:110 | Real PATCH vectors start not_set with America/New_York already saved; full persisted state must clear it and retain the explicit flag. Additional real select/detect/clear sequence checks (None, true) directly. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: appearance, not_set. |

## P0198: the layout marks an explicit Not set so the browser skips detection

Status: **closed**. Native identities:

- `controllers::users::profile_settings_tests::manual_timezone_choice_is_rendered_and_blocks_browser_detection`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:514` — `assert_response :success` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:74 | Real PATCH then GET then automatic detection; selected IANA option, saved zone, explicit Not set meta and detection refusal. Original selected IANA pre-save GET is also observed in original_profile_defaults_and_zone_options_match_rails_through_http. Cases: pages[America/New_York]. |
| `test/controllers/users/profiles_controller_test.rb:515` — `assert_select "meta[name=current-user-time-zone][content='']", count: 1` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:133 | Real PATCH then GET then automatic detection; selected IANA option, saved zone, explicit Not set meta and detection refusal. Original selected IANA pre-save GET is also observed in original_profile_defaults_and_zone_options_match_rails_through_http. Cases: pages[America/New_York]. |

## P0199: update rejects an unknown theme or time zone

Status: **closed**. Native identities:

- `controllers::users::profile_settings_tests::manual_profile_settings_match_pinned_rails_patch_vectors`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:520` — `assert_response :unprocessable_entity` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:256 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: bad_theme, bad_zone, bad_text. |
| `test/controllers/users/profiles_controller_test.rb:523` — `assert_response :unprocessable_entity` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:256 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: bad_theme, bad_zone, bad_text. |
| `test/controllers/users/profiles_controller_test.rb:526` — `assert_response :unprocessable_entity` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:256 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: bad_theme, bad_zone, bad_text. |
| `test/controllers/users/profiles_controller_test.rb:528` — `assert_equal "system", users(:david).reload.theme` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:281 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: bad_theme, bad_zone, bad_text. |
| `test/controllers/users/profiles_controller_test.rb:529` — `assert_equal "default", users(:david).text_size` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:281 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: bad_theme, bad_zone, bad_text. |
| `test/controllers/users/profiles_controller_test.rb:530` — `assert_nil users(:david).time_zone` | rust/crates/campfire/src/controllers/users/profile_settings_tests.rs:281 | Real original PUT and PATCH/CSRF; response status and redirect, complete saved settings/name/bio/other-user state, unchanged email. Invalid input renders a form and leaves the saved state unchanged. Both PUT (Rails original) and PATCH are executed; their independently generated Rails vector responses/states are identical. Cases: bad_theme, bad_zone, bad_text. |

## P0200: the layout carries the theme, time zone, and sound state

Status: **closed**. Native identities:

- `controllers::users::layout_preferences_tests::layout_theme_zone_and_manual_sound_state`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:538` — `assert_response :success` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:57 | Real GET; entire sound, Drive and zone meta lines match pinned Rails bytes, plus exact html theme/text-size attributes. Cache/user inputs match each original active/future/off scenario. Cases: manual_dnd. |
| `test/controllers/users/profiles_controller_test.rb:539` — `assert_select "html[data-theme=light]"` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:73 | Attributes on the actual opening html tag are checked, with exact cardinality; a matching substring in another tag is insufficient. Cases: manual_dnd. |
| `test/controllers/users/profiles_controller_test.rb:540` — `assert_select "html[data-text-size=large]"` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:73 | Attributes on the actual opening html tag are checked, with exact cardinality; a matching substring in another tag is insufficient. Cases: manual_dnd. |
| `test/controllers/users/profiles_controller_test.rb:541` — `assert_select "meta[name=color-scheme][content=light]", count: 1` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:119 | Real GET; entire sound, Drive and zone meta lines match pinned Rails bytes, plus exact html theme/text-size attributes. Cache/user inputs match each original active/future/off scenario. Cases: manual_dnd. |
| `test/controllers/users/profiles_controller_test.rb:542` — `assert_select "meta[name=current-user-time-zone][content='Pacific Time (US & Canada)']", count: 1` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Cases: manual_dnd. |
| `test/controllers/users/profiles_controller_test.rb:543` — `assert_select "meta[name=notification-dnd][content=muted]", count: 1` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Cases: manual_dnd. |

## P0201: the layout mutes sounds for the DND presence

Status: **closed**. Native identities:

- `controllers::users::layout_preferences_tests::layout_dnd_presence_mutes_sounds`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:551` — `assert_response :success` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:57 | Real GET; entire sound, Drive and zone meta lines match pinned Rails bytes, plus exact html theme/text-size attributes. Cache/user inputs match each original active/future/off scenario. Cases: dnd_presence. |
| `test/controllers/users/profiles_controller_test.rb:552` — `assert_select "meta[name=notification-dnd][content=muted]", count: 1` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Cases: dnd_presence. |

## P0202: the layout sends the quiet-hours window and zone for the sound gate

Status: **closed**. Native identities:

- `controllers::users::layout_preferences_tests::layout_quiet_hours_window_and_zone`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/profiles_controller_test.rb:561` — `assert_response :success` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:57 | Real GET; entire sound, Drive and zone meta lines match pinned Rails bytes, plus exact html theme/text-size attributes. Cache/user inputs match each original active/future/off scenario. Original setup retained: manual-only OOO/notifications kept, or quiet-hours disabled while stored zone remains UTC. Cases: quiet_hours, quiet_hours_off, original_quiet_hours_disabled. |
| `test/controllers/users/profiles_controller_test.rb:562` — `assert_select "meta[name=quiet-hours][content='1320-420']", count: 1` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Original setup retained: manual-only OOO/notifications kept, or quiet-hours disabled while stored zone remains UTC. Cases: quiet_hours, quiet_hours_off, original_quiet_hours_disabled. |
| `test/controllers/users/profiles_controller_test.rb:563` — `assert_select "meta[name=quiet-hours-zone][content='UTC']", count: 1` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Original setup retained: manual-only OOO/notifications kept, or quiet-hours disabled while stored zone remains UTC. Cases: quiet_hours, quiet_hours_off, original_quiet_hours_disabled. |
| `test/controllers/users/profiles_controller_test.rb:569` — `assert_response :success` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:57 | Real GET; entire sound, Drive and zone meta lines match pinned Rails bytes, plus exact html theme/text-size attributes. Cache/user inputs match each original active/future/off scenario. Original setup retained: manual-only OOO/notifications kept, or quiet-hours disabled while stored zone remains UTC. Cases: quiet_hours, quiet_hours_off, original_quiet_hours_disabled. |
| `test/controllers/users/profiles_controller_test.rb:570` — `assert_select "meta[name=quiet-hours]", count: 0` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Original setup retained: manual-only OOO/notifications kept, or quiet-hours disabled while stored zone remains UTC. Cases: quiet_hours, quiet_hours_off, original_quiet_hours_disabled. |
| `test/controllers/users/profiles_controller_test.rb:571` — `assert_select "meta[name=quiet-hours-zone]", count: 0` | rust/crates/campfire/src/controllers/users/layout_preferences_tests.rs:111 | All actual matching meta nodes, content values and cardinalities are compared with the exact Rails helper output. Duplicate or stray matching nodes fail. Original setup retained: manual-only OOO/notifications kept, or quiet-hours disabled while stored zone remains UTC. Cases: quiet_hours, quiet_hours_off, original_quiet_hours_disabled. |

## P0207: card shows identity, presence, role, and actions for a peer

Status: **closed**. Native identities:

- `controllers::users::people_tests::card_shows_identity_presence_role_and_actions_for_a_peer`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/cards_controller_test.rb:14` — `assert_response :success` | rust/crates/campfire/src/controllers/users/people_tests.rs:192 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: peer_online. |
| `test/controllers/users/cards_controller_test.rb:15` — `assert_select "turbo-frame#user_card", 1` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: peer_online. |
| `test/controllers/users/cards_controller_test.rb:16` — `assert_select ".profile-card__name", text: "Jason"` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: peer_online. |
| `test/controllers/users/cards_controller_test.rb:17` — `assert_select ".profile-card__presence", text: /Online/` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: peer_online. |
| `test/controllers/users/cards_controller_test.rb:18` — `assert_select ".profile-card__badge", text: "Admin"` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: peer_online. |
| `test/controllers/users/cards_controller_test.rb:19` — `assert_select "form[action='#{rooms_directs_path}']", 2` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: peer_online. |
| `test/controllers/users/cards_controller_test.rb:20` — `assert_select "button", text: "Message"` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: peer_online. |
| `test/controllers/users/cards_controller_test.rb:21` — `assert_select "button", text: "Start call"` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: peer_online. |
| `test/controllers/users/cards_controller_test.rb:22` — `assert_select "a", text: "Set a status", count: 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:205 | Actual routed card status-link cardinality, exact href and absence of data-turbo-frame, plus unchanged complete Rails DOM oracle. Cases: peer_online. |
| `test/controllers/users/cards_controller_test.rb:23` — `assert_select "a[href='#{user_path(users(:jason))}']", text: "View profile"` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: peer_online. |
| `test/controllers/users/cards_controller_test.rb:24` — `assert_select "button", text: "Copy mention"` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: peer_online. |

## P0208: offline peers read offline

Status: **closed**. Native identities:

- `controllers::users::people_tests::offline_peers_read_offline`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/cards_controller_test.rb:30` — `assert_response :success` | rust/crates/campfire/src/controllers/users/people_tests.rs:192 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: peer_offline. |
| `test/controllers/users/cards_controller_test.rb:31` — `assert_select ".profile-card__presence", text: /Offline/` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: peer_offline. |

## P0209: card shows the presence dot and custom status badge

Status: **closed**. Native identities:

- `controllers::users::people_tests::card_shows_presence_dot_and_custom_status_badge`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/cards_controller_test.rb:41` — `assert_response :success` | rust/crates/campfire/src/controllers/users/people_tests.rs:192 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: custom_status. |
| `test/controllers/users/cards_controller_test.rb:42` — `assert_select ".user-status-badge .avatar__presence[data-presence='online']", 1` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: custom_status. |
| `test/controllers/users/cards_controller_test.rb:43` — `assert_select ".user-status-badge__label", text: "Online"` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: custom_status. |
| `test/controllers/users/cards_controller_test.rb:44` — `assert_select ".user-status-badge__custom", text: "🚂 On a train"` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: custom_status. |

## P0210: your own card offers editing your profile instead

Status: **closed**. Native identities:

- `controllers::users::people_tests::own_card_offers_editing_your_profile`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/cards_controller_test.rb:50` — `assert_response :success` | rust/crates/campfire/src/controllers/users/people_tests.rs:192 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: own. |
| `test/controllers/users/cards_controller_test.rb:51` — `assert_select ".profile-card__name", text: "David"` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: own. |
| `test/controllers/users/cards_controller_test.rb:52` — `assert_select "a[href='#{user_profile_path}']", text: "Edit profile"` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: own. |
| `test/controllers/users/cards_controller_test.rb:53` — `assert_select "a[href='#{edit_user_status_path}']:not([data-turbo-frame])", text: "Set a status"` | rust/crates/campfire/src/controllers/users/people_tests.rs:205; rust/crates/campfire/src/controllers/users/people_tests.rs:207, rust/crates/campfire/src/controllers/users/people_tests.rs:208 | Actual routed card status-link cardinality, exact href and absence of data-turbo-frame, plus unchanged complete Rails DOM oracle. Cases: own. |
| `test/controllers/users/cards_controller_test.rb:54` — `assert_select "button", text: "Message", count: 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: own. |
| `test/controllers/users/cards_controller_test.rb:55` — `assert_select "button", text: "Start call", count: 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: own. |
| `test/controllers/users/cards_controller_test.rb:56` — `assert_select "button", text: "Copy mention", count: 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: own. |

## P0211: agents can be messaged but not called

Status: **closed**. Native identities:

- `controllers::users::people_tests::agents_can_be_messaged_but_not_called`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/cards_controller_test.rb:64` — `assert_response :success` | rust/crates/campfire/src/controllers/users/people_tests.rs:192 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: agent. |
| `test/controllers/users/cards_controller_test.rb:65` — `assert_select ".profile-card__badge", text: "Agent"` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: agent. |
| `test/controllers/users/cards_controller_test.rb:66` — `assert_select ".profile-card__owner", text: "Agent owned by David"` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: agent. |
| `test/controllers/users/cards_controller_test.rb:67` — `assert_select "button", text: "Message"` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: agent. |
| `test/controllers/users/cards_controller_test.rb:68` — `assert_select "button", text: "Start call", count: 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: agent. |

## P0212: inactive users show status without message actions

Status: **closed**. Native identities:

- `controllers::users::people_tests::inactive_users_show_status_without_message_actions`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/cards_controller_test.rb:76` — `assert_response :success` | rust/crates/campfire/src/controllers/users/people_tests.rs:192 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: deactivated. |
| `test/controllers/users/cards_controller_test.rb:77` — `assert_select ".profile-card__presence", text: /Deactivated/` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: deactivated. |
| `test/controllers/users/cards_controller_test.rb:78` — `assert_select "button", text: "Message", count: 0` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: deactivated. |
| `test/controllers/users/cards_controller_test.rb:79` — `assert_select "a[href='#{user_path(users(:kevin))}']", text: "View profile"` | rust/crates/campfire/src/controllers/users/people_tests.rs:159 | Real card GET; complete user_card DOM equals the pinned full-byte oracle, including every identity/presence/status/role/owner/action/absence assertion and element cardinality. Random CSRF value is not an original asserted value; all other attributes and text are retained. Cases: deactivated. |

## P0213: card requires sign-in

Status: **closed**. Native identities:

- `controllers::users::people_tests::cards_require_sign_in_and_unknown_people_are_not_found`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/cards_controller_test.rb:87` — `assert_redirected_to new_session_url` | rust/crates/campfire/src/controllers/users/people_tests.rs:308 | Actual anonymous GET redirects to the sign-in URL; signed-in unknown card is 404. |

## P0222: detects the browser zone when none is saved

Status: **closed**. Native identities:

- `controllers::users::preferences_tests::time_zone_detection_matches_rails_validation_and_saved_choice_vectors`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/time_zones_controller_test.rb:11` — `assert_response :success` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:40 | Actual PATCH; response JSON, saved zone/explicit flag/timestamp match Rails for original detect/saved/Not set/unknown inputs. |
| `test/controllers/users/time_zones_controller_test.rb:12` — `assert_equal "Pacific Time (US & Canada)", users(:david).reload.time_zone` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:64 | Actual PATCH; response JSON, saved zone/explicit flag/timestamp match Rails for original detect/saved/Not set/unknown inputs. |
| `test/controllers/users/time_zones_controller_test.rb:13` — `assert_equal "Pacific Time (US & Canada)", response.parsed_body["time_zone"]` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:45 | Actual PATCH; response JSON, saved zone/explicit flag/timestamp match Rails for original detect/saved/Not set/unknown inputs. |

## P0223: a hand-picked zone wins over later detections

Status: **closed**. Native identities:

- `controllers::users::preferences_tests::time_zone_detection_matches_rails_validation_and_saved_choice_vectors`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/time_zones_controller_test.rb:21` — `assert_response :success` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:40 | Actual PATCH; response JSON, saved zone/explicit flag/timestamp match Rails for original detect/saved/Not set/unknown inputs. |
| `test/controllers/users/time_zones_controller_test.rb:22` — `assert_equal "Eastern Time (US & Canada)", users(:david).reload.time_zone` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:64 | Actual PATCH; response JSON, saved zone/explicit flag/timestamp match Rails for original detect/saved/Not set/unknown inputs. |

## P0224: detection never overwrites an explicit choice, not even Not set

Status: **closed**. Native identities:

- `controllers::users::preferences_tests::time_zone_detection_matches_rails_validation_and_saved_choice_vectors`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/time_zones_controller_test.rb:30` — `assert_response :success` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:40 | Actual PATCH; response JSON, saved zone/explicit flag/timestamp match Rails for original detect/saved/Not set/unknown inputs. |
| `test/controllers/users/time_zones_controller_test.rb:31` — `assert_nil users(:david).reload.time_zone` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:64 | Actual PATCH; response JSON, saved zone/explicit flag/timestamp match Rails for original detect/saved/Not set/unknown inputs. |

## P0225: an unknown zone is ignored

Status: **closed**. Native identities:

- `controllers::users::preferences_tests::time_zone_detection_matches_rails_validation_and_saved_choice_vectors`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/users/time_zones_controller_test.rb:37` — `assert_response :success` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:40 | Actual PATCH; response JSON, saved zone/explicit flag/timestamp match Rails for original detect/saved/Not set/unknown inputs. |
| `test/controllers/users/time_zones_controller_test.rb:38` — `assert_nil users(:david).reload.time_zone` | rust/crates/campfire/src/controllers/users/preferences_tests.rs:64 | Actual PATCH; response JSON, saved zone/explicit flag/timestamp match Rails for original detect/saved/Not set/unknown inputs. |

## P0226: edit

Status: **closed**. Native identities:

- `controllers::accounts::view_tests::account_settings_body_navigation_and_footer_match_pinned_rails`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts_controller_test.rb:10` — `assert_response :ok` | rust/crates/campfire/src/controllers/accounts/view_tests.rs:94 | Real GET succeeds; complete page fragment byte oracle. |

## P0227: edit groups administrators separately from members with a divider

Status: **closed**. Native identities:

- `controllers::accounts::view_tests::account_settings_body_navigation_and_footer_match_pinned_rails`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts_controller_test.rb:16` — `assert_response :ok` | rust/crates/campfire/src/controllers/accounts/view_tests.rs:94 | Actual account_users frame: original divider exists, every original administrator row precedes it and every original member follows it. Complete page fragment byte oracle. |
| `test/controllers/accounts_controller_test.rb:19` — `assert_select "turbo-frame#account_users hr.separator.full-width"` | rust/crates/campfire/src/controllers/accounts/view_tests.rs:95 | Actual account_users frame: original divider exists, every original administrator row precedes it and every original member follows it. Complete page fragment byte oracle. |
| `test/controllers/accounts_controller_test.rb:30` — `assert divider_position, "Divider should exist in the response"` | rust/crates/campfire/src/controllers/accounts/view_tests.rs:95 | Actual account_users frame: original divider exists, every original administrator row precedes it and every original member follows it. Complete page fragment byte oracle. |
| `test/controllers/accounts_controller_test.rb:34` — `assert name_position, "Administrator #{name} should appear in the response"` | rust/crates/campfire/src/controllers/accounts/view_tests.rs:100 | Actual account_users frame: original divider exists, every original administrator row precedes it and every original member follows it. Complete page fragment byte oracle. |
| `test/controllers/accounts_controller_test.rb:35` — `assert name_position < divider_position, "Administrator #{name} should appear before the divider"` | rust/crates/campfire/src/controllers/accounts/view_tests.rs:100 | Actual account_users frame: original divider exists, every original administrator row precedes it and every original member follows it. Complete page fragment byte oracle. |
| `test/controllers/accounts_controller_test.rb:40` — `assert name_position, "Member #{name} should appear in the response"` | rust/crates/campfire/src/controllers/accounts/view_tests.rs:101 | Actual account_users frame: original divider exists, every original administrator row precedes it and every original member follows it. Complete page fragment byte oracle. |
| `test/controllers/accounts_controller_test.rb:41` — `assert name_position > divider_position, "Member #{name} should appear after the divider"` | rust/crates/campfire/src/controllers/accounts/view_tests.rs:101 | Actual account_users frame: original divider exists, every original administrator row precedes it and every original member follows it. Complete page fragment byte oracle. |

## P0228: update

Status: **closed**. Native identities:

- `controllers::accounts::mutation_tests::account_mutations_and_audits_match_pinned_rails_http_vectors`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts_controller_test.rb:46` — `assert users(:david).administrator?` | rust/crates/campfire/src/controllers/accounts/mutation_tests.rs:96 | The executing fixture boot omits OAuth credentials / signs in the administrator or member row; this is the original test precondition, not an extra product behavior. |
| `test/controllers/accounts_controller_test.rb:50` — `assert_redirected_to edit_account_url` | rust/crates/campfire/src/controllers/accounts/mutation_tests.rs:149 | Administrator role is the request fixture; actual PUT redirect and saved account name/audit state match Rails. |
| `test/controllers/accounts_controller_test.rb:51` — `assert_equal accounts(:signal).name, "Different"` | rust/crates/campfire/src/controllers/accounts/mutation_tests.rs:178 | Administrator role is the request fixture; actual PUT redirect and saved account name/audit state match Rails. |

## P0229: non-admins cannot update

Status: **closed**. Native identities:

- `controllers::accounts::mutation_tests::account_and_ban_mutations_authorize_before_writes_and_audits`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/accounts_controller_test.rb:56` — `assert users(:kevin).member?` | rust/crates/campfire/src/controllers/accounts/mutation_tests.rs:11 | The executing fixture boot omits OAuth credentials / signs in the administrator or member row; this is the original test precondition, not an extra product behavior. |
| `test/controllers/accounts_controller_test.rb:59` — `assert_response :forbidden` | rust/crates/campfire/src/controllers/accounts/mutation_tests.rs:46 | Member role is the request fixture; actual PUT is forbidden with no state/audit writes. |

## P0272: service worker serves as JavaScript with the fetch and notification handlers

Status: **closed**. Native identities:

- `controllers::pwa::tests::pwa_http_bodies_match_rails_before_and_after_first_run`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/pwa_controller_test.rb:7` — `assert_response :success` | rust/crates/campfire/src/controllers/pwa.rs:56 | Real unsigned HTTP service-worker/offline requests; full response status/content-type/body byte oracle establishes every original source clause. Offline cookie absence additionally checked. |
| `test/controllers/pwa_controller_test.rb:8` — `assert_equal "text/javascript", response.media_type` | rust/crates/campfire/src/controllers/pwa.rs:57 | Real unsigned HTTP service-worker/offline requests; full response status/content-type/body byte oracle establishes every original source clause. Offline cookie absence additionally checked. |
| `test/controllers/pwa_controller_test.rb:11` — `assert_includes body, 'addEventListener("fetch"'` | rust/crates/campfire/src/controllers/pwa.rs:58 | Real unsigned HTTP service-worker/offline requests; full response status/content-type/body byte oracle establishes every original source clause. Offline cookie absence additionally checked. |
| `test/controllers/pwa_controller_test.rb:12` — `assert_includes body, 'addEventListener("push"'` | rust/crates/campfire/src/controllers/pwa.rs:58 | Real unsigned HTTP service-worker/offline requests; full response status/content-type/body byte oracle establishes every original source clause. Offline cookie absence additionally checked. |
| `test/controllers/pwa_controller_test.rb:13` — `assert_includes body, 'addEventListener("notificationclick"'` | rust/crates/campfire/src/controllers/pwa.rs:58 | Real unsigned HTTP service-worker/offline requests; full response status/content-type/body byte oracle establishes every original source clause. Offline cookie absence additionally checked. |

## P0273: service worker caches static assets only

Status: **closed**. Native identities:

- `controllers::pwa::tests::pwa_http_bodies_match_rails_before_and_after_first_run`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/pwa_controller_test.rb:19` — `assert_response :success` | rust/crates/campfire/src/controllers/pwa.rs:56 | Real unsigned HTTP service-worker/offline requests; full response status/content-type/body byte oracle establishes every original source clause. Offline cookie absence additionally checked. |
| `test/controllers/pwa_controller_test.rb:25` — `assert_includes body, 'url.pathname === OFFLINE_URL &#124;&#124; url.pathname.startsWith("/assets/")'` | rust/crates/campfire/src/controllers/pwa.rs:58 | Real unsigned HTTP service-worker/offline requests; full response status/content-type/body byte oracle establishes every original source clause. Offline cookie absence additionally checked. |
| `test/controllers/pwa_controller_test.rb:26` — `assert_includes body, "networkThenOffline"` | rust/crates/campfire/src/controllers/pwa.rs:58 | Real unsigned HTTP service-worker/offline requests; full response status/content-type/body byte oracle establishes every original source clause. Offline cookie absence additionally checked. |
| `test/controllers/pwa_controller_test.rb:27` — `assert_equal 1, body.scan("cache.put").size` | rust/crates/campfire/src/controllers/pwa.rs:58 | Real unsigned HTTP service-worker/offline requests; full response status/content-type/body byte oracle establishes every original source clause. Offline cookie absence additionally checked. |

## P0274: notification clicks focus an existing window before opening a new one

Status: **closed**. Native identities:

- `controllers::pwa::tests::pwa_http_bodies_match_rails_before_and_after_first_run`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/pwa_controller_test.rb:33` — `assert_response :success` | rust/crates/campfire/src/controllers/pwa.rs:56 | Real unsigned HTTP service-worker/offline requests; full response status/content-type/body byte oracle establishes every original source clause. Offline cookie absence additionally checked. |
| `test/controllers/pwa_controller_test.rb:36` — `assert_includes body, "clients.matchAll"` | rust/crates/campfire/src/controllers/pwa.rs:58 | Real unsigned HTTP service-worker/offline requests; full response status/content-type/body byte oracle establishes every original source clause. Offline cookie absence additionally checked. |
| `test/controllers/pwa_controller_test.rb:37` — `assert_includes body, "existing.focus()"` | rust/crates/campfire/src/controllers/pwa.rs:58 | Real unsigned HTTP service-worker/offline requests; full response status/content-type/body byte oracle establishes every original source clause. Offline cookie absence additionally checked. |
| `test/controllers/pwa_controller_test.rb:38` — `assert_includes body, "clients.openWindow(url)"` | rust/crates/campfire/src/controllers/pwa.rs:58 | Real unsigned HTTP service-worker/offline requests; full response status/content-type/body byte oracle establishes every original source clause. Offline cookie absence additionally checked. |

## P0275: offline shell renders signed-out with reconnect behavior

Status: **closed**. Native identities:

- `controllers::pwa::tests::pwa_http_bodies_match_rails_before_and_after_first_run`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/pwa_controller_test.rb:44` — `assert_response :success` | rust/crates/campfire/src/controllers/pwa.rs:56 | Real unsigned HTTP service-worker/offline requests; full response status/content-type/body byte oracle establishes every original source clause. Offline cookie absence additionally checked. |
| `test/controllers/pwa_controller_test.rb:45` — `assert_includes response.body, "You&rsquo;re offline &mdash; reconnecting&hellip;"` | rust/crates/campfire/src/controllers/pwa.rs:58 | Real unsigned HTTP service-worker/offline requests; full response status/content-type/body byte oracle establishes every original source clause. Offline cookie absence additionally checked. |
| `test/controllers/pwa_controller_test.rb:46` — `assert_includes response.body, "offline-retry"` | rust/crates/campfire/src/controllers/pwa.rs:58 | Real unsigned HTTP service-worker/offline requests; full response status/content-type/body byte oracle establishes every original source clause. Offline cookie absence additionally checked. |
| `test/controllers/pwa_controller_test.rb:47` — `assert_includes response.body, 'addEventListener("online"'` | rust/crates/campfire/src/controllers/pwa.rs:58 | Real unsigned HTTP service-worker/offline requests; full response status/content-type/body byte oracle establishes every original source clause. Offline cookie absence additionally checked. |
| `test/controllers/pwa_controller_test.rb:48` — `assert_no_match(/session_token/, response.headers["Set-Cookie"].to_s)` | rust/crates/campfire/src/controllers/pwa.rs:59 | Real unsigned HTTP service-worker/offline requests; full response status/content-type/body byte oracle establishes every original source clause. Offline cookie absence additionally checked. |

## P0276: service worker fetch and notification logic

Status: **closed**. Native identities:

- `controllers::pwa::tests::original_service_worker_logic_checks_the_real_http_script`

| Rails assertion | Discriminating Rust assertion | Observation and oracle cases |
| --- | --- | --- |
| `test/controllers/pwa_controller_test.rb:58` — `assert $?.success?, output` | rust/crates/campfire/src/controllers/pwa.rs:100 | Normal enabled Rust test starts the actual application, GETs the worker, runs the byte-identical original Rails Node harness on that HTTP body, and asserts subprocess success and all-checks stdout. No byte-only stand-in. Cases: Pinned original Node fetch/push/notification harness. |
| `test/controllers/pwa_controller_test.rb:59` — `assert_includes output, "all checks passed"` | rust/crates/campfire/src/controllers/pwa.rs:105 | Normal enabled Rust test starts the actual application, GETs the worker, runs the byte-identical original Rails Node harness on that HTTP body, and asserts subprocess success and all-checks stdout. No byte-only stand-in. Cases: Pinned original Node fetch/push/notification harness. |

