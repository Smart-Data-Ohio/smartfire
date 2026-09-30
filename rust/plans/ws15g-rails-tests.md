# WS15g Rails test coverage — partial

Reference: `d7c7de92`. 426 Rails cases in 31 files: 176 mapped to Rust assertions; 250 explicitly deferred.

These are domain-level ports grouped into Rust tests, not executions of the original Ruby tests. Webhook HTTP ingestion, transactional enqueue, fetch persistence/runtime handler and the shared stuck-claim sweep with runtime periodic registration are covered. Notifier posting/dedupe/privacy/thread routing with its registered runtime and message broadcasts are also covered. PR card broadcasts and view/system parity, other HTTP controllers, the remaining PR model/reference/thread/subscription domain and agent write HTTP controllers remain deferred. All deferred cases retain WS15g as owner; WS11 supplies the agent authentication seam and outbound event-webhook runtime. No coverage or parity allowlist has been added.

| Rails file | Cases passing grouped assertions | Deferred |
|---|---:|---:|
| `test/controllers/agents/github/pull_request_actions_controller_test.rb` | 0/21 | 21 |
| `test/helpers/github_pull_requests_helper_test.rb` | 0/20 | 20 |
| `test/controllers/rooms/github_subscriptions_controller_test.rb` | 0/17 | 17 |
| `test/integration/github_pr_cards_test.rb` | 0/16 | 16 |
| `test/controllers/github/pull_request_review_requests_controller_test.rb` | 0/15 | 15 |
| `test/controllers/rooms/github/pull_request_cards_controller_test.rb` | 0/15 | 15 |
| `test/controllers/github/app_connections_controller_test.rb` | 0/13 | 13 |
| `test/controllers/github/connections_controller_test.rb` | 0/13 | 13 |
| `test/controllers/github/pull_request_comments_controller_test.rb` | 0/11 | 11 |
| `test/models/github/pull_request_test.rb` | 6/17 | 11 |
| `test/models/github/pull_request_thread_test.rb` | 0/11 | 11 |
| `test/controllers/accounts/bots/github_connections_controller_test.rb` | 0/10 | 10 |
| `test/models/github/repository_subscription_test.rb` | 0/10 | 10 |
| `test/controllers/agents/github_action_delivery_test.rb` | 0/9 | 9 |
| `test/controllers/github/pull_request_reviews_controller_test.rb` | 0/9 | 9 |
| `test/integration/github_pr_threads_test.rb` | 0/9 | 9 |
| `test/controllers/github/pull_request_threads_controller_test.rb` | 0/7 | 7 |
| `test/jobs/audit_log_github_execution_test.rb` | 0/6 | 6 |
| `test/controllers/github/pull_request_write_actions_controller_test.rb` | 0/5 | 5 |
| `test/models/github/review_logins_test.rb` | 0/5 | 5 |
| `test/controllers/github/webhooks_controller_test.rb` | 18/21 | 3 |
| `test/jobs/github/deliver_subscription_event_job_test.rb` | 34/37 | 3 |
| `test/models/github/notification_test.rb` | 0/3 | 3 |
| `test/system/github_pr_write_actions_test.rb` | 0/3 | 3 |
| `test/jobs/github/fetch_pull_request_job_test.rb` | 20/22 | 2 |
| `test/jobs/github/perform_agent_action_job_test.rb` | 34/36 | 2 |
| `test/models/github/write_client_test.rb` | 12/13 | 1 |
| `test/models/github/agent_pull_request_action_test.rb` | 11/11 | 0 |
| `test/models/github/app_test.rb` | 11/11 | 0 |
| `test/models/github/pull_request_url_test.rb` | 10/10 | 0 |
| `test/models/github_connected_account_test.rb` | 20/20 | 0 |

## `test/controllers/accounts/bots/github_connections_controller_test.rb` (10 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| an administrator can link the agent's account | Deferred; WS15g continuation | — |
| the owner without admin rights can neither link, relink, nor unlink | Deferred; WS15g continuation | — |
| another member gets 403 linking and unlinking | Deferred; WS15g continuation | — |
| an administrator can unlink the agent's account | Deferred; WS15g continuation | — |
| a rejected token stores nothing and shows the GitHub message | Deferred; WS15g continuation | — |
| an unreachable GitHub shows a retry message | Deferred; WS15g continuation | — |
| a blank token is rejected | Deferred; WS15g continuation | — |
| linking again after a disconnect replaces the token and clears the reason | Deferred; WS15g continuation | — |
| the bot page shows the login without ever rendering the token | Deferred; WS15g continuation | — |
| deactivating the bot disconnects its GitHub account like a human's | Deferred; WS15g continuation | — |

## `test/controllers/agents/github/pull_request_actions_controller_test.rb` (21 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| a bad credential is 401 | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| a suspended agent is 401 | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| a room the agent is not a member of is 404 | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| an unknown room is 404 | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| a pull request the room does not discuss is 404 | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| a mapping in another room is 404 | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| a missing pull_request_id is 404 | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| a missing external_action grant is 403 with the approvals error shape | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| an external_action grant in another room is 403 | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| an agent without a linked GitHub account is 422 | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| an agent with a disconnected account is 422 | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| a comment without a body is 422 with field errors | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| request_changes without a body is 422 with field errors | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| request_review without valid logins is 422 with field errors | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| an unknown kind is 422 with field errors | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| a legacy bot key is rejected | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| a human session is rejected | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| a comment request creates one approval and returns 202 without calling GitHub | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| approve, request_changes, and request_review build their own payloads | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| reviewers also accept an array | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| a repeated external_id returns the existing row with 200 | Deferred; WS15g continuation; WS11 owns authentication middleware | — |

## `test/controllers/agents/github_action_delivery_test.rb` (9 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| an approved action records a completed ledger row with the GitHub url | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| a failed action records the reason without a url | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| completion appears in event polling with the github_action payload | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| failed completions poll with the message and no url | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| ack works on completion rows | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| completion posts the webhook with agent and github_action keys | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| completion rows are readable by their own agent only | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| the ledger page lists the completion with its status | Deferred; WS15g continuation; WS11 owns authentication middleware | — |
| the ledger page lists failures with their reason | Deferred; WS15g continuation; WS11 owns authentication middleware | — |

## `test/controllers/github/app_connections_controller_test.rb` (13 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| connect redirects to the GitHub App authorize URL | Deferred; WS15g continuation | — |
| connect answers 404 while the App is unconfigured | Deferred; WS15g continuation | — |
| callback exchanges the code and stores an app token | Deferred; WS15g continuation | — |
| callback with a stale state sends the member back to try again | Deferred; WS15g continuation | — |
| callback when GitHub reports an error | Deferred; WS15g continuation | — |
| callback answers 404 while the App is unconfigured | Deferred; WS15g continuation | — |
| disconnect revokes the app token remotely | Deferred; WS15g continuation | — |
| disconnect refreshes an expired token before revoking the grant | Deferred; WS15g continuation | — |
| disconnect proceeds when the refresh fails | Deferred; WS15g continuation | — |
| disconnect proceeds when revocation fails | Deferred; WS15g continuation | — |
| disconnect sends no revocation for a PAT | Deferred; WS15g continuation | — |
| linking a PAT over an app connection resets the source | Deferred; WS15g continuation | — |
| reconnecting through the app revokes only the previous app token | Deferred; WS15g continuation | — |

## `test/controllers/github/connections_controller_test.rb` (13 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| linking validates the token with GET /user and stores the login | Deferred; WS15g continuation | — |
| linking sets the profile username when blank | Deferred; WS15g continuation | — |
| linking replaces a differing profile username with the verified one | Deferred; WS15g continuation | — |
| linking releases the login from a member who claimed it without verification | Deferred; WS15g continuation | — |
| linking never takes a login another member's linked token verifies | Deferred; WS15g continuation | — |
| the profile cannot edit the login while a verified account is linked | Deferred; WS15g continuation | — |
| the profile edits the login again once the link is disconnected | Deferred; WS15g continuation | — |
| a rejected token stores nothing | Deferred; WS15g continuation | — |
| linking again after a disconnect replaces the token and clears the reason | Deferred; WS15g continuation | — |
| unlinking destroys the account | Deferred; WS15g continuation | — |
| profile shows link and unlink state without rendering the token | Deferred; WS15g continuation | — |
| linking never logs the pasted token | Deferred; WS15g continuation | — |
| the token parameter is filtered from request logs | Deferred; WS15g continuation | — |

## `test/controllers/github/pull_request_comments_controller_test.rb` (11 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| posts the comment with the member's token, never the workspace token | Deferred; WS15g continuation | — |
| success over Turbo Stream replaces the frame with the confirmation | Deferred; WS15g continuation | — |
| non-members get not found | Deferred; WS15g continuation | — |
| a member without a linked token gets the connect prompt | Deferred; WS15g continuation | — |
| a member with a disconnected token gets the reconnect prompt | Deferred; WS15g continuation | — |
| a GitHub 403 renders inline with no retry | Deferred; WS15g continuation | — |
| a failed comment keeps the body for retry | Deferred; WS15g continuation | — |
| a GitHub 401 disconnects the account and shows the reconnect prompt | Deferred; WS15g continuation | — |
| a blank body is rejected without calling GitHub | Deferred; WS15g continuation | — |
| a PR the room does not discuss gets not found | Deferred; WS15g continuation | — |
| posting never logs the member's token | Deferred; WS15g continuation | — |

## `test/controllers/github/pull_request_review_requests_controller_test.rb` (15 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| requests the review with the member's token, never the workspace token | Deferred; WS15g continuation | — |
| reviewers are split on commas or whitespace, stripped of @, downcased, and deduped | Deferred; WS15g continuation | — |
| success over Turbo Stream replaces the frame with the confirmation | Deferred; WS15g continuation | — |
| non-members get not found | Deferred; WS15g continuation | — |
| a member without a linked token gets the connect prompt | Deferred; WS15g continuation | — |
| a member with a disconnected token gets the reconnect prompt | Deferred; WS15g continuation | — |
| a GitHub 403 renders inline with no retry | Deferred; WS15g continuation | — |
| a GitHub 422 keeps the submitted reviewers and shows the message | Deferred; WS15g continuation | — |
| a GitHub 401 disconnects the account and shows the reconnect prompt | Deferred; WS15g continuation | — |
| invalid logins are rejected without calling GitHub and keep the input | Deferred; WS15g continuation | — |
| empty input is rejected without calling GitHub | Deferred; WS15g continuation | — |
| more than 15 reviewers are rejected without calling GitHub | Deferred; WS15g continuation | — |
| a PR the room does not discuss gets not found | Deferred; WS15g continuation | — |
| a bot key is forbidden, exactly as the comments endpoint | Deferred; WS15g continuation | — |
| requesting never logs the member's token | Deferred; WS15g continuation | — |

## `test/controllers/github/pull_request_reviews_controller_test.rb` (9 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| approve posts with the member's token, never the workspace token | Deferred; WS15g continuation | — |
| request-changes posts the review body | Deferred; WS15g continuation | — |
| request-changes without a body is rejected locally with 422 | Deferred; WS15g continuation | — |
| an unknown event is rejected without calling GitHub | Deferred; WS15g continuation | — |
| non-members get not found | Deferred; WS15g continuation | — |
| a member without a linked token gets the connect prompt | Deferred; WS15g continuation | — |
| a GitHub 403 renders inline with no retry | Deferred; WS15g continuation | — |
| a failed review keeps the body for retry | Deferred; WS15g continuation | — |
| a GitHub 401 disconnects the account and shows the reconnect prompt | Deferred; WS15g continuation | — |

## `test/controllers/github/pull_request_threads_controller_test.rb` (7 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| discuss creates a thread with the card message as parent and records the mapping | Deferred; WS15g continuation | — |
| discuss reuses the room's existing thread for the PR | Deferred; WS15g continuation | — |
| discuss reuses the winner and drops the loser when the race is lost at the unique index | Deferred; WS15g continuation | — |
| discuss reuses the winner and drops the loser when the race is lost at the validation | Deferred; WS15g continuation | — |
| non-members get not found | Deferred; WS15g continuation | — |
| a message that does not reference the PR gets not found | Deferred; WS15g continuation | — |
| a thread reply cannot parent a discussion | Deferred; WS15g continuation | — |

## `test/controllers/github/pull_request_write_actions_controller_test.rb` (5 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| a linked member gets the composer and review buttons | Deferred; WS15g continuation | — |
| a member without a linked token gets the connect prompt | Deferred; WS15g continuation | — |
| a member with a disconnected token gets the reconnect prompt | Deferred; WS15g continuation | — |
| non-members get not found | Deferred; WS15g continuation | — |
| the thread header carries the write-actions frame | Deferred; WS15g continuation | — |

## `test/controllers/github/webhooks_controller_test.rb` (21 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| valid pull_request signature updates the record and broadcasts once | Deferred; WS15g continuation | — |
| response carries no card content | Mapped to grouped Rust assertions; WS15g | `webhook_http_status_body_selection_and_privacy_match_rails` |
| pull_request webhook stores repository privacy when the payload carries it | Mapped to grouped Rust assertions; WS15g | `webhook_http_status_body_selection_and_privacy_match_rails` |
| pull_request webhook stores a public repository when the payload says so | Mapped to grouped Rust assertions; WS15g | `webhook_http_status_body_selection_and_privacy_match_rails` |
| pull_request webhook leaves privacy alone when the payload omits it | Mapped to grouped Rust assertions; WS15g | `webhook_http_status_body_selection_and_privacy_match_rails` |
| bad signature is rejected without enqueueing | Mapped to grouped Rust assertions; WS15g | `webhook_security_rejects_bad_or_missing_signatures_before_parsing` |
| missing signature is rejected | Mapped to grouped Rust assertions; WS15g | `webhook_security_rejects_bad_or_missing_signatures_before_parsing` |
| missing secret answers unavailable | Mapped to grouped Rust assertions; WS15g | `webhook_security_missing_or_blank_secret_is_unavailable` |
| redelivered events are ignored | Mapped to grouped Rust assertions; WS15g | `webhook_redelivered_supported_events_do_not_enqueue_again` |
| issue_comment on a referenced PR enqueues exactly one refresh | Mapped to grouped Rust assertions; WS15g | `webhook_http_status_body_selection_and_privacy_match_rails` |
| issue_comment enqueues only the refresh, never subscription delivery | Mapped to grouped Rust assertions; WS15g | `webhook_http_status_body_selection_and_privacy_match_rails` |
| redelivered issue_comment events enqueue nothing | Mapped to grouped Rust assertions; WS15g | `webhook_redelivered_supported_events_do_not_enqueue_again` |
| issue_comment on plain issues and unreferenced PRs is ignored | Mapped to grouped Rust assertions; WS15g | `webhook_http_status_body_selection_and_privacy_match_rails` |
| events for unreferenced PRs are ignored | Mapped to grouped Rust assertions; WS15g | `webhook_http_status_body_selection_and_privacy_match_rails` |
| unhandled event types are acknowledged and ignored | Mapped to grouped Rust assertions; WS15g | `webhook_http_status_body_selection_and_privacy_match_rails` |
| pull_request_review, check_run, check_suite, and status events enqueue a refresh | Mapped to grouped Rust assertions; WS15g | `webhook_http_status_body_selection_and_privacy_match_rails` |
| mixed-case repository names in payloads still find the stored PR | Mapped to grouped Rust assertions; WS15g | `webhook_http_status_body_selection_and_privacy_match_rails` |
| subscribed repositories enqueue subscription delivery and post once | Deferred; WS15g continuation | — |
| redelivered subscription events post nothing | Deferred; WS15g continuation | — |
| unsubscribed repositories enqueue no delivery and create no bot user | Mapped to grouped Rust assertions; WS15g | `webhook_http_status_body_selection_and_privacy_match_rails` |
| check events without PR links and status events for other branches are ignored | Mapped to grouped Rust assertions; WS15g | `webhook_http_status_body_selection_and_privacy_match_rails` |

## `test/controllers/rooms/github/pull_request_cards_controller_test.rb` (15 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| a member whose token can read the repository sees the card | Deferred; WS15g continuation | — |
| a member without a linked account gets the empty frame and no GitHub request | Deferred; WS15g continuation | — |
| a member with a disconnected account gets the empty frame and no GitHub request | Deferred; WS15g continuation | — |
| GitHub 404 gives the empty frame | Deferred; WS15g continuation | — |
| GitHub 401 marks the account disconnected and gives the empty frame | Deferred; WS15g continuation | — |
| the decision is cached per viewer and repository | Deferred; WS15g continuation | — |
| a cached denial no longer applies after the member relinks | Deferred; WS15g continuation | — |
| a transport error renders the empty frame and caches nothing | Deferred; WS15g continuation | — |
| a public PR renders the card with no GitHub request | Deferred; WS15g continuation | — |
| a thread frame renders the card and files summary when the viewer may see it | Deferred; WS15g continuation | — |
| a thread frame is empty when the viewer may not see it | Deferred; WS15g continuation | — |
| non-members get not found | Deferred; WS15g continuation | — |
| a message from another room is not found | Deferred; WS15g continuation | — |
| a message that does not reference the PR is not found | Deferred; WS15g continuation | — |
| missing context is not found | Deferred; WS15g continuation | — |

## `test/controllers/rooms/github_subscriptions_controller_test.rb` (17 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| administrator can subscribe a room with default events | Deferred; WS15g continuation | — |
| administrator can subscribe with an explicit event selection | Deferred; WS15g continuation | — |
| subscribing an open room returns to its edit page | Deferred; WS15g continuation | — |
| room creator can subscribe without being an administrator | Deferred; WS15g continuation | — |
| duplicate and malformed subscriptions redirect with an alert | Deferred; WS15g continuation | — |
| administrator can change events and remove a subscription | Deferred; WS15g continuation | — |
| removing one of several subscriptions keeps the bot in the room | Deferred; WS15g continuation | — |
| plain members get forbidden | Deferred; WS15g continuation | — |
| non-members get not found | Deferred; WS15g continuation | — |
| direct rooms get not found | Deferred; WS15g continuation | — |
| github section renders for administrators but not plain members | Deferred; WS15g continuation | — |
| subscribing checks access with the subscriber's own token and records a verified reader | Deferred; WS15g continuation | — |
| a subscriber whose token cannot read the repository is refused | Deferred; WS15g continuation | — |
| a subscriber without a linked GitHub account is refused | Deferred; WS15g continuation | — |
| a disconnected GitHub account cannot vouch for a subscription | Deferred; WS15g continuation | — |
| administrators may override the check, leaving the subscription unverified | Deferred; WS15g continuation | — |
| the override checkbox renders for administrators only | Deferred; WS15g continuation | — |

## `test/helpers/github_pull_requests_helper_test.rb` (20 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| cache key changes when a referenced pull request is updated | Deferred; WS15g continuation | — |
| cache key for a message without pull requests is just the message | Deferred; WS15g continuation | — |
| cache key changes when a thread reply is posted | Deferred; WS15g continuation | — |
| cache key changes when an older thread reply is deleted | Deferred; WS15g continuation | — |
| cache key reads the reply count without a query | Deferred; WS15g continuation | — |
| cache key carries the streaming flag | Deferred; WS15g continuation | — |
| cache key changes when a step is added to the message | Deferred; WS15g continuation | — |
| cache key changes when a quoted source is edited | Deferred; WS15g continuation | — |
| cache key changes when a quoted source's author is renamed | Deferred; WS15g continuation | — |
| cache key changes when a quoted source's room is renamed | Deferred; WS15g continuation | — |
| cache key changes when a quoted source is deleted | Deferred; WS15g continuation | — |
| cache key changes when a poll is voted and retracted | Deferred; WS15g continuation | — |
| cache key carries the system note flag | Deferred; WS15g continuation | — |
| cache key changes when the message is pinned and unpinned | Deferred; WS15g continuation | — |
| cache key changes on unpin even when a referenced card is newer | Deferred; WS15g continuation | — |
| cache key changes when a referenced X post is fetched | Deferred; WS15g continuation | — |
| cache key changes when a referenced link embed is fetched | Deferred; WS15g continuation | — |
| cache key changes when a referenced event is updated | Deferred; WS15g continuation | — |
| pr cards still render when the queue is down | Deferred; WS15g continuation | — |
| a failed pr enqueue releases its fetch claim | Deferred; WS15g continuation | — |

## `test/integration/github_pr_cards_test.rb` (16 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| a message with a PR link renders the card | Deferred; WS15g continuation | — |
| the card keeps the fetched repository name case | Deferred; WS15g continuation | — |
| a message without a PR link renders no card | Deferred; WS15g continuation | — |
| an unfetched public PR renders a loading card | Deferred; WS15g continuation | — |
| a failed fetch renders an error card | Deferred; WS15g continuation | — |
| a private PR renders only the empty lazy frame in the message HTML | Deferred; WS15g continuation | — |
| a PR with unknown privacy is treated as private | Deferred; WS15g continuation | — |
| rendering a stale card enqueues a refresh | Deferred; WS15g continuation | — |
| a loaded card with no check data shows No checks | Deferred; WS15g continuation | — |
| rendering a room page costs no extra queries per message with a PR link | Deferred; WS15g continuation | — |
| one render enqueues a single refresh for one stale PR linked by many messages | Deferred; WS15g continuation | — |
| repeat views by different users enqueue at most one refresh per PR per window | Deferred; WS15g continuation | — |
| a fresh card does not enqueue a refresh on render | Deferred; WS15g continuation | — |
| a non-member cannot see the card through the room | Deferred; WS15g continuation | — |
| the open-room join page leaks no card content to non-members | Deferred; WS15g continuation | — |
| added routes never render card content to unauthorized callers | Deferred; WS15g continuation | — |

## `test/integration/github_pr_threads_test.rb` (9 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| a card without a thread shows a Discuss button | Deferred; WS15g continuation | — |
| a card with a thread links to it | Deferred; WS15g continuation | — |
| a PR thread shows the card and files summary above its messages | Deferred; WS15g continuation | — |
| the files summary omits the more line when everything is shown | Deferred; WS15g continuation | — |
| a PR thread without fetched files shows a loading summary | Deferred; WS15g continuation | — |
| an ordinary thread shows no PR header | Deferred; WS15g continuation | — |
| file paths from the API render as text | Deferred; WS15g continuation | — |
| a PR thread for a private PR shows only the lazy frame in its header | Deferred; WS15g continuation | — |
| a non-member cannot open the PR thread | Deferred; WS15g continuation | — |

## `test/jobs/audit_log_github_execution_test.rb` (6 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| a completed GitHub action is recorded with the decider as actor | Deferred; WS15g continuation | — |
| a refused execution is recorded as failed without a GitHub request | Deferred; WS15g continuation | — |
| a retried job records no second row | Deferred; WS15g continuation | — |
| a failing audit write still enqueues the outcome webhook | Deferred; WS15g continuation | — |
| a failing sweep audit write still enqueues the outcome webhook | Deferred; WS15g continuation | — |
| a stuck claim recovered by the sweep is recorded as failed | Deferred; WS15g continuation | — |

## `test/jobs/github/deliver_subscription_event_job_test.rb` (37 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| opened posts one bot message with the pr url and reference | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| the posted url is built from the subscribed repository, not the payload | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| webhook text cannot smuggle a mention token into the post | Mapped to grouped Rust assertions; WS15g | `github_notifier_security_redacts_per_subscription_and_neutralizes_mentions` |
| reopened, ready for review, and synchronize post nothing after opened | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| reopened posts when the pr opened before the subscription | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| closed with merged true posts merged, merged false posts closed | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| review_requested posts and records an inbox item for the linked member | Deferred; WS15g continuation (Notifier source/item/preference assertions covered; WS12 owns inbox accessible_to and the general mention recorder) | — |
| review_requested records nothing for non-members or unlinked logins | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| review_requested skips the item when the reviewer switched them off | Deferred; WS15g continuation (Notifier source/item/preference assertions covered; WS12 owns inbox accessible_to and the general mention recorder) | — |
| review_requested still notifies a member with notifications off but not an invisible one | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| team review requests post nothing | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| review_submitted posts each review once | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| three check failures on one sha post once, a new sha posts again | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| successful checks post nothing | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| failed status posts for the stored pr on that branch | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| failed status without a stored pr posts nothing | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| unsubscribed event keys post nothing | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| unsubscribed repositories post nothing and create no bot user | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| unhandled events post nothing | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| posted messages broadcast to the room like any other message | Mapped to grouped Rust assertions; WS15g | `github_notifier_durable_handler_publishes_real_room_and_thread_frames` |
| claimed notifications record their message | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| an event posts a thread reply where the PR has a thread and a room message elsewhere | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| thread updates dedupe like room messages | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| review_requested in a PR thread points the inbox item at the thread message | Deferred; WS15g continuation (Notifier source/item/preference assertions covered; WS12 owns inbox accessible_to and the general mention recorder) | — |
| an update for a locked PR thread falls back to a root room message | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| an update for a closed PR thread still lands in the thread | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| subscription events find the PR thread regardless of payload case | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| failed checks use the stored title regardless of payload case | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| failed status matches stored PRs regardless of payload case | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| thread updates broadcast to the thread stream | Mapped to grouped Rust assertions; WS15g | `github_notifier_durable_handler_publishes_real_room_and_thread_frames` |
| posts nothing to a soft-deleted room | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| records no inbox item for a soft-deleted room | Mapped to grouped Rust assertions; WS15g | `github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails` |
| a private repository posts without the title to a subscription with no verified reader | Mapped to grouped Rust assertions; WS15g | `github_notifier_security_redacts_per_subscription_and_neutralizes_mentions` |
| a repository whose privacy the payload omits is treated as private | Mapped to grouped Rust assertions; WS15g | `github_notifier_security_redacts_per_subscription_and_neutralizes_mentions` |
| a private repository keeps the title for a verified reader's subscription | Mapped to grouped Rust assertions; WS15g | `github_notifier_security_redacts_per_subscription_and_neutralizes_mentions` |
| failed checks on a private repository omit the stored title for an unverified subscription | Mapped to grouped Rust assertions; WS15g | `github_notifier_security_redacts_per_subscription_and_neutralizes_mentions` |
| one webhook redacts per subscription | Mapped to grouped Rust assertions; WS15g | `github_notifier_security_redacts_per_subscription_and_neutralizes_mentions` |

## `test/jobs/github/fetch_pull_request_job_test.rb` (22 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| success stores card fields, clears errors, and stamps fetched_at | Mapped to grouped Rust assertions; WS15g | `github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails` |
| fetch stores the repository privacy from base.repo.private | Mapped to grouped Rust assertions; WS15g | `github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails` |
| fetch stores a public repository as not private | Mapped to grouped Rust assertions; WS15g | `github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails` |
| fetch leaves privacy unknown when the payload omits base.repo.private | Mapped to grouped Rust assertions; WS15g | `github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails` |
| merged, closed, and draft states map to card states | Mapped to grouped Rust assertions; WS15g | `github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails` |
| changes requested wins over approvals | Mapped to grouped Rust assertions; WS15g | `github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails` |
| failing check runs map to failing | Mapped to grouped Rust assertions; WS15g | `github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails` |
| no check runs and no statuses leaves check_status blank | Mapped to grouped Rust assertions; WS15g | `github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails` |
| pending combined status with real statuses maps to pending | Mapped to grouped Rust assertions; WS15g | `github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails` |
| a draft with approvals shows its review decision | Mapped to grouped Rust assertions; WS15g | `github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails` |
| a draft with no reviews shows review required | Mapped to grouped Rust assertions; WS15g | `github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails` |
| 404 leaves a fetch_error and stamps fetched_at without raising | Mapped to grouped Rust assertions; WS15g | `github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails` |
| rate limiting leaves a fetch_error without raising | Mapped to grouped Rust assertions; WS15g | `github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails` |
| network errors leave a fetch_error without raising | Mapped to grouped Rust assertions; WS15g | `github_fetch_transport_failure_persists_error_without_changing_card_or_files` |
| sends the workspace token when configured and omits it otherwise | Mapped to grouped Rust assertions; WS15g | `github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails` |
| a later success clears an earlier fetch_error | Mapped to grouped Rust assertions; WS15g | `github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails` |
| changed files are fetched only for PRs with a thread mapping | Mapped to grouped Rust assertions; WS15g | `github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails` |
| mapped PRs store the files summary without diff bodies | Mapped to grouped Rust assertions; WS15g | `github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails` |
| the files summary caps at 100 files with the PR total | Mapped to grouped Rust assertions; WS15g | `github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails` |
| a failed files fetch keeps the previous summary and sets fetch_error | Mapped to grouped Rust assertions; WS15g | `github_fetch_persisted_fields_errors_reviews_checks_and_files_match_rails` |
| card updates broadcast the thread header to mapped thread streams | Deferred; WS15g continuation | — |
| card updates broadcast nothing without referencing messages or mappings | Deferred; WS15g continuation | — |

## `test/jobs/github/perform_agent_action_job_test.rb` (36 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| approving a github action enqueues the job, denying does not | Deferred; WS15g continuation | — |
| a relinked GitHub account after approval makes no request | Mapped to grouped Rust assertions; WS15g | `github_agent_rechecks_payload_and_linked_identity_before_any_write` |
| a replaced GitHub connection with the same login makes no request | Mapped to grouped Rust assertions; WS15g | `github_agent_rechecks_payload_and_linked_identity_before_any_write` |
| an approval that recorded no GitHub identity makes no request | Mapped to grouped Rust assertions; WS15g | `github_agent_rechecks_payload_and_linked_identity_before_any_write` |
| approving a non-github action enqueues nothing | Deferred; WS15g continuation | — |
| an approved comment posts with the agent token and records completion | Mapped to grouped Rust assertions; WS15g | `github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails` |
| an approved approve posts the review with the agent token | Mapped to grouped Rust assertions; WS15g | `github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails` |
| approved request_changes posts the review body | Mapped to grouped Rust assertions; WS15g | `github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails` |
| approved request_review posts the reviewer logins | Mapped to grouped Rust assertions; WS15g | `github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails` |
| a denied approval makes no request and records failure | Mapped to grouped Rust assertions; WS15g | `github_agent_rechecks_authority_before_any_write` |
| an expired approval makes no request and records failure | Mapped to grouped Rust assertions; WS15g | `github_agent_rechecks_authority_before_any_write` |
| a cancelled approval makes no request and records failure | Mapped to grouped Rust assertions; WS15g | `github_agent_rechecks_authority_before_any_write` |
| removed membership makes no request and records failure | Mapped to grouped Rust assertions; WS15g | `github_agent_rechecks_authority_before_any_write` |
| a revoked grant makes no request and records failure | Mapped to grouped Rust assertions; WS15g | `github_agent_rechecks_authority_before_any_write` |
| a suspended agent makes no request and records failure | Mapped to grouped Rust assertions; WS15g | `github_agent_rechecks_authority_before_any_write` |
| a deleted thread mapping makes no request and records failure | Mapped to grouped Rust assertions; WS15g | `github_agent_rechecks_authority_before_any_write` |
| a disconnected account makes no request and records failure | Mapped to grouped Rust assertions; WS15g | `github_agent_rechecks_payload_and_linked_identity_before_any_write` |
| a destroyed account makes no request and records failure | Mapped to grouped Rust assertions; WS15g | `github_agent_rechecks_payload_and_linked_identity_before_any_write` |
| a GitHub 401 disconnects the account and records failure | Mapped to grouped Rust assertions; WS15g | `github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails` |
| a GitHub 403 records failure with GitHub's message | Mapped to grouped Rust assertions; WS15g | `github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails` |
| an approval whose payload describes a different action than its summary makes no request | Mapped to grouped Rust assertions; WS15g | `github_agent_rechecks_payload_and_linked_identity_before_any_write` |
| an approval whose payload names a different pull request than its summary makes no request | Mapped to grouped Rust assertions; WS15g | `github_agent_rechecks_payload_and_linked_identity_before_any_write` |
| running the job twice for one approval posts once | Mapped to grouped Rust assertions; WS15g | `github_agent_concurrent_duplicates_share_one_claim_and_one_outbound_write` |
| a historical completion row with a NULL approval column still suppresses a second run | Mapped to grouped Rust assertions; WS15g | `github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails` |
| a duplicate enqueue that slips past the check posts no second request | Mapped to grouped Rust assertions; WS15g | `github_agent_concurrent_duplicates_share_one_claim_and_one_outbound_write` |
| a job that loses the execution claim makes no GitHub request | Mapped to grouped Rust assertions; WS15g | `github_agent_concurrent_duplicates_share_one_claim_and_one_outbound_write` |
| a failed GitHub call rewrites the winner's claim as a failure | Mapped to grouped Rust assertions; WS15g | `github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails` |
| completion rows are unique per agent and approval | Mapped to grouped Rust assertions; WS15g | `github_agent_completion_index_is_scoped_to_agent_approval_and_event_type` |
| a failed run is not retried by a second run | Mapped to grouped Rust assertions; WS15g | `github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails` |
| a missing approval and a non-github approval are silent no-ops | Mapped to grouped Rust assertions; WS15g | `github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails` |
| a running claim older than 15 minutes is marked failed by the sweeper | Mapped to grouped Rust assertions; WS15g | `github_claim_persisted_outcomes_and_audits_match_pinned_rails` |
| a claim the sweep already failed is not rewritten by a late finish | Mapped to grouped Rust assertions; WS15g | `github_claim_sweep_and_late_finish_keep_the_first_outcome_and_enqueue_once` |
| a claim finished while the sweep runs keeps its result | Mapped to grouped Rust assertions; WS15g | `github_claim_sweep_and_late_finish_keep_the_first_outcome_and_enqueue_once` |
| a fresh running claim is left alone by the sweeper | Mapped to grouped Rust assertions; WS15g | `github_claim_sweep_fails_only_overdue_running_claims_and_preserves_metadata` |
| an approved action posts with the owner's app token when available | Mapped to grouped Rust assertions; WS15g | `github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails` |
| an owner PAT never overrides the agent account | Mapped to grouped Rust assertions; WS15g | `github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails` |

## `test/models/github/agent_pull_request_action_test.rb` (11 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| comment requires a body | Mapped to grouped Rust assertions; WS15g | `github_agent_action_validation_summary_payload_and_normalization_match_rails` |
| request_changes requires a body | Mapped to grouped Rust assertions; WS15g | `github_agent_action_validation_summary_payload_and_normalization_match_rails` |
| approve accepts a missing body | Mapped to grouped Rust assertions; WS15g | `github_agent_action_validation_summary_payload_and_normalization_match_rails` |
| request_review requires 1 to 15 valid logins | Mapped to grouped Rust assertions; WS15g | `github_agent_action_validation_summary_payload_and_normalization_match_rails` |
| request_review normalises logins like the human endpoint | Mapped to grouped Rust assertions; WS15g | `github_agent_action_validation_summary_payload_and_normalization_match_rails` |
| unknown kinds are rejected | Mapped to grouped Rust assertions; WS15g | `github_agent_action_validation_summary_payload_and_normalization_match_rails` |
| oversized bodies are rejected before the approval payload limit | Mapped to grouped Rust assertions; WS15g | `github_agent_action_validation_summary_payload_and_normalization_match_rails` |
| summaries name the action and the pull request | Mapped to grouped Rust assertions; WS15g | `github_agent_action_validation_summary_payload_and_normalization_match_rails` |
| payload carries pull_request_id, kind, body, and reviewers | Mapped to grouped Rust assertions; WS15g | `github_agent_action_validation_summary_payload_and_normalization_match_rails` |
| from_payload rebuilds a valid action | Mapped to grouped Rust assertions; WS15g | `github_agent_action_validation_summary_payload_and_normalization_match_rails` |
| perform calls the same client methods as the human controllers | Mapped to grouped Rust assertions; WS15g | `github_agent_persisted_outcomes_audits_jobs_and_requests_match_rails` |

## `test/models/github/app_test.rb` (11 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| unconfigured without both credentials | Mapped to grouped Rust assertions; WS15g | `app_requires_both_credentials_and_authorizes_with_empty_scope` |
| authorize_url points at github.com with the client id | Mapped to grouped Rust assertions; WS15g | `app_requires_both_credentials_and_authorizes_with_empty_scope` |
| exchange_code returns the token response | Mapped to grouped Rust assertions; WS15g | `oauth_response_matrix_matches_rails` |
| exchange_code raises Unauthorized on invalid_grant | Mapped to grouped Rust assertions; WS15g | `oauth_response_matrix_matches_rails` |
| exchange_code raises Unauthorized on a 200 body carrying bad_verification_code | Mapped to grouped Rust assertions; WS15g | `oauth_response_matrix_matches_rails` |
| refresh_access_token raises Unauthorized on a 200 body carrying bad_refresh_token | Mapped to grouped Rust assertions; WS15g | `rejected_refreshes_disconnect_but_transient_failures_preserve_credentials_and_timestamp` |
| exchange_code raises Error on transport failure | Mapped to grouped Rust assertions; WS15g | `transport_errors_and_revocations_never_expose_credentials` |
| refresh_access_token posts the refresh grant | Mapped to grouped Rust assertions; WS15g | `refresh_sends_rotating_grant` |
| revoke_token deletes one token and never raises | Mapped to grouped Rust assertions; WS15g | `revoke_distinguishes_token_from_grant_and_uses_basic_auth` |
| revoke_grant deletes the whole authorization | Mapped to grouped Rust assertions; WS15g | `revoke_distinguishes_token_from_grant_and_uses_basic_auth` |
| revoke_grant treats an unknown token as failure, revoke_token as gone | Mapped to grouped Rust assertions; WS15g | `revoke_distinguishes_token_from_grant_and_uses_basic_auth` |

## `test/models/github/notification_test.rb` (3 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| claim! wins once per subscription and dedupe key | Deferred; WS15g continuation | — |
| claim! is scoped to the subscription | Deferred; WS15g continuation | — |
| claim! survives a duplicate insert race | Deferred; WS15g continuation | — |

## `test/models/github/pull_request_test.rb` (17 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| for_reference upserts by owner, repo, and number | Deferred; WS15g continuation | — |
| repository names are stored downcased so links in any case share one row | Deferred; WS15g continuation | — |
| display_full_name keeps the fetched repository name case | Deferred; WS15g continuation | — |
| display_full_name falls back to the stored names | Deferred; WS15g continuation | — |
| collapse_case_duplicates! merges case variants onto the lowest id and repoints links and threads | Deferred; WS15g continuation | — |
| stale? is true until fetched and after ten minutes | Deferred; WS15g continuation | — |
| claim_fetch_request! grants one fetch per PR per ten minutes | Deferred; WS15g continuation | — |
| claiming a fetch request does not broadcast a card update | Deferred; WS15g continuation | — |
| with_rendering_details preloads referenced PRs | Deferred; WS15g continuation | — |
| creating a message with a PR URL references the PR and enqueues a fetch | Mapped to grouped Rust assertions; WS15g | `github_message_create_and_edit_hooks_reconcile_references_and_fetches` |
| duplicate URLs in one message create a single reference | Mapped to grouped Rust assertions; WS15g | `github_message_create_and_edit_hooks_reconcile_references_and_fetches` |
| a message without a PR URL references nothing and enqueues nothing | Mapped to grouped Rust assertions; WS15g | `github_message_create_and_edit_hooks_reconcile_references_and_fetches` |
| URLs in code spans and fenced blocks create no references | Mapped to grouped Rust assertions; WS15g | `github_message_reference_security_ignores_code_and_caps_before_case_normalization` |
| editing a message to add a PR URL adds the reference | Mapped to grouped Rust assertions; WS15g | `github_message_create_and_edit_hooks_reconcile_references_and_fetches` |
| editing a message to remove a PR URL drops the reference | Mapped to grouped Rust assertions; WS15g | `github_message_create_and_edit_hooks_reconcile_references_and_fetches` |
| updating a record broadcasts a card replace to each referencing room once | Deferred; WS15g continuation | — |
| card broadcasts carry the title for a public pull request and only a frame for a private one | Deferred; WS15g continuation | — |

## `test/models/github/pull_request_thread_test.rb` (11 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| one thread per PR per room, and a thread discusses at most one PR | Deferred; WS15g continuation | — |
| the same PR can be discussed in different rooms | Deferred; WS15g continuation | — |
| create_or_reuse! creates the mapping once | Deferred; WS15g continuation | — |
| create_or_reuse! reuses the winner when a concurrent insert loses | Deferred; WS15g continuation | — |
| create_or_reuse! reuses the winner when the validation runs after the winner commits | Deferred; WS15g continuation | — |
| create_or_reuse! reraises validation errors other than the PR-per-room uniqueness | Deferred; WS15g continuation | — |
| create_or_reuse! reraises when the PR uniqueness failure is not the only error | Deferred; WS15g continuation | — |
| the unique index rejects a duplicate mapping without validations | Deferred; WS15g continuation | — |
| destroying the thread destroys the mapping | Deferred; WS15g continuation | — |
| payload_for_message carries the PR object in a PR thread | Deferred; WS15g continuation | — |
| payload_for_message is null in other threads and room messages | Deferred; WS15g continuation | — |

## `test/models/github/pull_request_url_test.rb` (10 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| extracts a canonical pull request URL | Mapped to grouped Rust assertions; WS15g | `github_url_extraction_and_non_code_html_match_pinned_rails` |
| extracts /pulls/ variants and trailing paths, queries, and fragments | Mapped to grouped Rust assertions; WS15g | `github_url_extraction_and_non_code_html_match_pinned_rails` |
| extracts multiple URLs and deduplicates repeats | Mapped to grouped Rust assertions; WS15g | `github_url_extraction_and_non_code_html_match_pinned_rails` |
| ignores non-PR URLs | Mapped to grouped Rust assertions; WS15g | `github_url_extraction_and_non_code_html_match_pinned_rails` |
| extract caps references at Twitter's MAX_PER_MESSAGE | Mapped to grouped Rust assertions; WS15g | `github_url_extraction_and_non_code_html_match_pinned_rails` |
| non_code_text drops code spans and fenced blocks but keeps prose and labeled links | Mapped to grouped Rust assertions; WS15g | `github_url_extraction_and_non_code_html_match_pinned_rails` |
| non_code_text keeps a URL at a br or block boundary matchable | Mapped to grouped Rust assertions; WS15g | `github_url_extraction_and_non_code_html_match_pinned_rails` |
| pull_request_url? matches only PR URLs | Mapped to grouped Rust assertions; WS15g | `github_url_extraction_and_non_code_html_match_pinned_rails` |
| ignores dot-only owner and repo segments | Mapped to grouped Rust assertions; WS15g | `github_url_extraction_and_non_code_html_match_pinned_rails` |
| still matches names containing dots and dashes | Mapped to grouped Rust assertions; WS15g | `github_url_extraction_and_non_code_html_match_pinned_rails` |

## `test/models/github/repository_subscription_test.rb` (10 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| new subscriptions default to the standard event selection | Deferred; WS15g continuation | — |
| event keys are validated against the known set | Deferred; WS15g continuation | — |
| owner and repo are stripped and downcased | Deferred; WS15g continuation | — |
| owner and repo follow pull request url character rules | Deferred; WS15g continuation | — |
| one subscription per room and repository, case-insensitively | Deferred; WS15g continuation | — |
| clearing every event on an existing subscription is rejected | Deferred; WS15g continuation | — |
| direct rooms cannot be subscribed | Deferred; WS15g continuation | — |
| subscribing adds the github bot to the room | Deferred; WS15g continuation | — |
| the github bot joins only subscribed rooms | Deferred; WS15g continuation | — |
| destroying the last subscription removes the bot from the room | Deferred; WS15g continuation | — |

## `test/models/github/review_logins_test.rb` (5 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| splits on commas and whitespace, strips @, downcases, and dedupes | Deferred; WS15g continuation | — |
| accepts an array of tokens | Deferred; WS15g continuation | — |
| blank input normalizes to an empty array | Deferred; WS15g continuation | — |
| invalid logins normalize to nil | Deferred; WS15g continuation | — |
| more than 15 unique logins normalizes to nil | Deferred; WS15g continuation | — |

## `test/models/github/write_client_test.rb` (13 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| authenticated_login returns the token owner's login | Mapped to grouped Rust assertions; WS15g | `write_paths_payloads_identity_and_headers_match_rails` |
| authenticated_login raises Unauthorized on 401 | Mapped to grouped Rust assertions; WS15g | `repository_access_denies_refusals_but_propagates_unauthorized_and_errors` |
| create_issue_comment posts to the issues comments endpoint | Mapped to grouped Rust assertions; WS15g | `write_paths_payloads_identity_and_headers_match_rails` |
| create_review posts approve and request-changes events | Mapped to grouped Rust assertions; WS15g | `write_paths_payloads_identity_and_headers_match_rails` |
| request_reviewers posts the logins to the requested_reviewers endpoint | Mapped to grouped Rust assertions; WS15g | `write_paths_payloads_identity_and_headers_match_rails` |
| request_reviewers maps a GitHub 422 to Refused with GitHub's message | Mapped to grouped Rust assertions; WS15g | `write_status_matrix_matches_rails_and_cannot_inject_mentions` |
| 401 raises Unauthorized | Mapped to grouped Rust assertions; WS15g | `write_status_matrix_matches_rails_and_cannot_inject_mentions` |
| 403 and 404 raise Refused with GitHub's message | Mapped to grouped Rust assertions; WS15g | `write_status_matrix_matches_rails_and_cannot_inject_mentions` |
| network errors raise Error without logging the token | Deferred; WS15g continuation (warning-log assertion; error/privacy assertions already covered) | — |
| repository_readable? is true when GitHub answers 200 | Mapped to grouped Rust assertions; WS15g | `repository_access_denies_refusals_but_propagates_unauthorized_and_errors` |
| repository_readable? is false on 403 and 404 | Mapped to grouped Rust assertions; WS15g | `repository_access_denies_refusals_but_propagates_unauthorized_and_errors` |
| repository_readable? raises Unauthorized on 401 | Mapped to grouped Rust assertions; WS15g | `repository_access_denies_refusals_but_propagates_unauthorized_and_errors` |
| repository_readable? raises Error on network failure | Mapped to grouped Rust assertions; WS15g | `transport_errors_and_revocations_never_expose_credentials` |

## `test/models/github_connected_account_test.rb` (20 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| one account per user | Mapped to grouped Rust assertions; WS15g | `accounts_enforce_rails_validations` |
| token is encrypted at rest | Mapped to grouped Rust assertions; WS15g | `accounts_encrypt_both_columns_and_read_rails_rows` |
| an undecryptable token is unusable, not fatal | Mapped to grouped Rust assertions; WS15g | `unreadable_or_tampered_tokens_disconnect_without_panicking` |
| connected, usable, and disconnect reason | Mapped to grouped Rust assertions; WS15g | `pats_are_used_as_is_and_disconnected_accounts_are_unusable` |
| a fresh PAT is used as-is | Mapped to grouped Rust assertions; WS15g | `pats_are_used_as_is_and_disconnected_accounts_are_unusable` |
| refresh token is encrypted at rest | Mapped to grouped Rust assertions; WS15g | `accounts_encrypt_both_columns_and_read_rails_rows` |
| an expired app token refreshes in place | Mapped to grouped Rust assertions; WS15g | `expired_app_tokens_rotate_with_early_refresh_and_keep_old_refresh_if_omitted` |
| a rejected refresh disconnects the account | Mapped to grouped Rust assertions; WS15g | `rejected_refreshes_disconnect_but_transient_failures_preserve_credentials_and_timestamp` |
| a bad_refresh_token response disconnects the account | Mapped to grouped Rust assertions; WS15g | `rejected_refreshes_disconnect_but_transient_failures_preserve_credentials_and_timestamp` |
| a stale instance reuses the rotated token without refreshing again | Mapped to grouped Rust assertions; WS15g | `stale_account_lookups_reuse_rotated_credentials_without_http` |
| a rejected refresh keeps a token another process already rotated | Mapped to grouped Rust assertions; WS15g | `refresh_races_reuse_the_winner_without_holding_a_database_transaction` |
| the refresh HTTP call runs with no refresh transaction open | Mapped to grouped Rust assertions; WS15g | `refresh_races_reuse_the_winner_without_holding_a_database_transaction` |
| two concurrent refreshes converge on one token | Mapped to grouped Rust assertions; WS15g | `refresh_races_reuse_the_winner_without_holding_a_database_transaction` |
| a failed refresh transport records last_error and keeps the old token | Mapped to grouped Rust assertions; WS15g | `transport_failure_leaves_app_connected_and_skips_remote_revoke` |
| revoke_remote_token refreshes an expired token first | Mapped to grouped Rust assertions; WS15g | `disconnect_refreshes_before_revoking_the_whole_grant` |
| revoke_remote_token records last_error and skips the revoke when the refresh fails | Mapped to grouped Rust assertions; WS15g | `failed_refresh_skips_grant_revocation_but_records_error` |
| agent identity prefers the owner's usable app token | Mapped to grouped Rust assertions; WS15g | `agent_identity_prefers_owner_app_but_falls_back_to_machine_pat` |
| agent identity falls back to the agent account without an owner app token | Mapped to grouped Rust assertions; WS15g | `agent_identity_prefers_owner_app_but_falls_back_to_machine_pat` |
| agent identity falls back to the agent PAT after a bad_refresh_token | Mapped to grouped Rust assertions; WS15g | `rejected_owner_refresh_falls_back_to_agent_pat` |
| agent identity falls back when the owner app token is disconnected | Mapped to grouped Rust assertions; WS15g | `agent_identity_prefers_owner_app_but_falls_back_to_machine_pat` |

## `test/system/github_pr_write_actions_test.rb` (3 tests)

| Rails test | Status and owner | Rust coverage |
|---|---|---|
| a linked member comments from a PR thread and sees the inline confirmation | Deferred; WS15g continuation | — |
| a linked member requests a review from a PR thread and sees the inline confirmation | Deferred; WS15g continuation | — |
| a member without a linked token sees the connect prompt in the thread | Deferred; WS15g continuation | — |
