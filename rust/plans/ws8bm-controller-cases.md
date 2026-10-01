# WS8bm named Rails controller case inventory

Pinned reference: `d7c7de92`. This is a case attribution backlog, not a claim that the cases are unimplemented. The report records independently executed Rust aggregate tests and Rails reference counts separately. Entries with explicit evidence below are attributed; other entries still require case-level Rust attribution/signoff. 17 further root-controller declarations now have explicit existing-test attribution; this records assertion scope, not new test executions. No one-to-one port count is claimed. Browser/system execution remains deferred.

## test/controllers/messages_controller_test.rb

56 named declarations; 18 have scoped Rust evidence below, and 38 await attribution/signoff. Reference execution counts are in the main report.

- index returns the last page by default — WS8bm. Attributed to `messages::paging_tests::pages_match_rails_tuple_edges_formats_and_etag_bytes` (complete pinned Rails page/response comparisons).
- index is not found for a soft-deleted room — WS8bm. Attributed to `messages::http_tests::deleted_room_is_inaccessible_even_with_a_lingering_membership` (actual requests with retained membership).
- index returns a page before the specified message — WS8bm. Attributed to `messages::paging_tests::pages_match_rails_tuple_edges_formats_and_etag_bytes` (actual before-cursor requests against Rails).
- index returns a page after the specified message — WS8bm. Attributed to `messages::paging_tests::pages_match_rails_tuple_edges_formats_and_etag_bytes` (actual after-cursor requests against Rails).
- index returns no_content when there are no messages — WS8bm. Attributed to `messages::paging_tests::pages_match_rails_tuple_edges_formats_and_etag_bytes` (empty-edge status and exact response headers).
- index etag changes when an off-page reply source is edited — WS8bm.
- index etag changes when a card fetch completes — WS15g / WS15e; WS8bm render integration.
- index etag changes when an author is renamed — WS8bm. Attributed to `messages::paging_tests::validators_observe_related_rows_and_older_unpins_without_message_touches` (author cache stamp invalidates the existing validator).
- index etag changes when the older of two pins is removed — WS8b-m2; WS8bm cache integration.
- get renders a single message belonging to the user — WS8bm. Attributed to `messages::root_tests::standalone_message_wrapper_matches_rails_bytes` (four complete standalone Rails view bodies).
- room message list announces live appends — WS8b-r; WS8bm list integration.
- image attachments use the filename as alt text — WS8bm.
- creating a message broadcasts the message to the room — WS8bm.
- broadcast message actions preserve a nonstandard request port — WS8bm.
- creating a Markdown message preserves its source and derives the rich body — WS8bm.
- preview renders the same safe Markdown without writing — WS8bm. Attributed to `messages::http_tests::preview_matches_real_rails_http_without_writing` (eight complete Rails responses and unchanged message count).
- preview requires room membership — WS8bm. Attributed to `messages::http_tests::preview_requires_membership_and_a_valid_source_parameter` (signed-in non-member receives 404).
- preview rejects oversized Markdown without parsing or writing it — WS8bm. Attributed to `messages::http_tests::preview_matches_real_rails_http_without_writing` (actual oversized Rails source case and unchanged message count).
- preview is protected against cross-site form submissions — WS8bm. Attributed to `messages::http_tests::preview_rejects_cross_site_submissions_without_a_csrf_token` (actual hostile-Origin request receives 422).
- creating a message broadcasts unread room to each member — WS8bm.
- creating a message doesn't broadcast unread room to non-members — WS8bm.
- update updates a message belonging to the user — WS8bm. Attributed to `messages::root_tests::updates_match_rails_json_and_saved_rows_including_legacy_conversion` (six exact HTTP update responses and saved fields).
- updating a Markdown message preserves exact new source — WS8bm. Attributed to `messages::root_tests::updates_match_rails_json_and_saved_rows_including_legacy_conversion` (exact persisted markdown_source).
- a legacy body update clears stale Markdown mode — WS8bm. Attributed to `messages::root_tests::updates_match_rails_json_and_saved_rows_including_legacy_conversion` (legacy conversion and clearing state).
- editing a message to add a PR URL broadcasts the new card — WS15g / WS15e; WS8bm render integration.
- editing a message to remove a PR URL broadcasts an empty card container — WS15g / WS15e; WS8bm render integration.
- legacy rich-text edits re-sync card references — WS15g / WS15e; WS8bm render integration.
- messages render empty card containers for future broadcasts — WS15g / WS15e; WS8bm render integration.
- admin cannot update a message belonging to another user — WS8bm. Attributed to `messages::http_tests::author_only_edits_even_for_an_administrator` (admin edit/update denied).
- destroy destroys a message belonging to the user — WS8bm. Attributed to `messages::paging_tests::root_formats_and_destroy_side_effects_match_rails` (own deletes across Rails formats remove rows).
- admin destroy destroys a message belonging to another user — WS8bm.
- destroy broadcasts tombstone updates to replies — WS8bm.
- destroying a thread parent broadcasts a thread summary refresh — WS8bm.
- edited messages show an edited marker with the edit time — WS8bm.
- the edited marker renders the same UTC time in every time zone — WS8bm.
- editing a message broadcasts its meta so other clients see the edited marker — WS8bm.
- identical and attachment-only saves do not mark a message edited — WS8bm.
- identical rich-text saves do not mark a message edited — WS8bm.
- a blank body on a bodyless message does not mark it edited — WS8bm.
- formatting-only rich-text edits still mark a message edited — WS8bm.
- reactions do not mark a message edited — WS8bm.
- card fetches do not mark a message edited — WS15g / WS15e; WS8bm render integration.
- reply tombstones do not mark the reply edited — WS8bm.
- ensure non-admin can't update a message belonging to another user — WS8bm.
- ensure non-admin can't destroy a message belonging to another user — WS8bm. Attributed to `messages::http_tests::non_author_non_admin_cannot_edit_or_delete` (other member delete denied).
- mentioning a bot triggers a webhook — WS11; WS8bm HTTP integration.
- mentioning a bot from Markdown triggers a webhook — WS11; WS8bm HTTP integration.
- mentioning an agent-backed bot skips the legacy webhook job — WS11; WS8bm HTTP integration.
- mentioning an agent-backed bot posts exactly one webhook with the agent key — WS11; WS8bm HTTP integration.
- revoked agent delivery posts no webhook — WS11; WS8bm HTTP integration.
- mentioning a bot without an agent row still uses the legacy webhook — WS11; WS8bm HTTP integration.
- retried create with the same client id returns the original message — WS8bm. Attributed to `messages::review_tests::scalar_retry_paths_match_rails_bytes_and_rows`: complete responses/rows for true, false, numeric, string and blank IDs; actual pinned Rails oracle.
- system notes render as a compact note without message chrome — WS8bm.
- system notes cannot be edited or deleted by their actor — WS8bm.
- system notes cannot be deleted by an administrator — WS8bm.
- system note actions report no edit or delete — WS8bm.

## test/controllers/messages_drive_attachments_test.rb

19 named declarations; case-level Rust attribution/signoff pending. Reference execution counts are in the main report.

- create with drive_file_ids stores them in order — WS8bm.
- create with attachments and no text is valid — WS8bm.
- create deduplicates repeated ids and strips blanks — WS8bm.
- create with an invalid id answers 422 and creates nothing — WS8bm.
- create with more than 10 attachments answers 422 — WS8bm.
- update with a new set replaces the stored set — WS8bm.
- update with a submitted set broadcasts the attachments block to the room — WS8bm.
- update without the key does not broadcast the attachments block — WS8bm.
- update with a scalar drive_file_ids answers 422 and keeps the stored set — WS8bm.
- update without the key leaves the set alone — WS8bm.
- update with only the blank sentinel removes all attachments — WS8bm.
- removing every attachment from a textless message answers 422 and keeps the set — WS8bm.
- update with an invalid id answers 422 and keeps the stored set — WS8bm.
- a non-creator cannot change attachments — WS8bm.
- JSON message shape includes drive_attachments with file_id and url only — WS8bm.
- JSON message shape carries an empty drive_attachments array without attachments — WS8bm.
- rendered message carries the generic chip with the open link and no file name — WS8bm.
- viewers with and without Drive consent receive identical attachment markup — WS14g; WS8bm attachment rendering.
- edit form lists attachments as removable chips with the blank sentinel — WS8bm.

## test/controllers/messages/cached_fragment_csrf_test.rb

4 named declarations; case-level Rust attribution/signoff pending. Reference execution counts are in the main report.

- a cached message page serves no viewer's tokens to the next — WS8bm.
- a cached refresh serves no viewer's tokens to the next — WS8bm.
- a cached thread page serves no viewer's tokens to the next — WS8bm.
- every form in a cached message submits with the page's header token — WS8bm.

## test/controllers/messages/legacy_presentation_cache_test.rb

2 named declarations; case-level Rust attribution/signoff pending. Reference execution counts are in the main report.

- fragments cached before the autolink fix aren't served after it — WS8bm.
- pages validated before the autolink fix aren't revalidated after it — WS8bm.

## test/controllers/messages/boosts_controller_test.rb

17 named declarations. Three cases have the component/action evidence below; merged room/browser signoff and the remaining named attribution stay pending. Reference execution counts are in the main report.

- create — WS8bm.
- destroy — WS8bm.
- a human emoji toggle removes legacy duplicates under the message lock — WS8bm.
- a quick reaction sent as a shortcode toggles instead of duplicating — WS8bm.
- create accepts a brand shortcode and renders its icon — WS8bm.
- create accepts a workspace icon shortcode and renders its image — WS8bm.
- create stores an unknown shortcode as literal text — WS8bm.
- two users reacting with the same custom icon share one counted chip — WS8bm.
- an autocompleted emoji shortcode with a trailing space renders its chip — WS8bm.
- an autocompleted brand shortcode with a trailing space renders its icon chip — WS8bm.
- a non-quick emoji aggregates and toggles instead of duplicating — WS8bm.
- keycaps, flags, ZWJ sequences, VS16 and modifiers aggregate and toggle — WS8bm.
- plain digits and letters stay per-person legacy boosts without toggling — WS8bm.
- free text stays a per-person legacy boost without toggling — WS8bm.
- the boost action opens the soft keyboard — WS8bm; `boost_pages_match_complete_rails_forms_distinct_counts_and_escaped_reactors`, complete `index_empty`/`index_mixed` Rails components with `soft-keyboard#open` and actual successful index requests. Keyboard interaction remains system-phase work.
- action metadata groups reaction counts by distinct reactor — WS8bm; the same test's actual `actions_david`/`actions_jason` requests compare whole JSON with three duplicate boosts, two distinct reactors and viewer-specific activity.
- the reaction tooltip lists reactors as plain text, never interactive content — WS8bm; the same test's complete `index_hostile_name` component escapes an interactive-looking reactor name. Merged room HTTP/browser signoff remains pending; this component comparison alone is not that signoff.

## test/controllers/channel_threads_controller_test.rb

24 named declarations; case-level Rust attribution/signoff pending. Reference execution counts are in the main report.

- creation accepts nested thread message parameters and joins only the creator — WS8bm.
- retried creation with the same first-message client id returns the existing thread — WS8bm. Attributed to `messages::review_tests::scalar_retry_paths_match_rails_bytes_and_rows`: complete responses/rows for true, false, numeric, string and blank IDs; actual pinned Rails oracle.
- creator settings, joined-member reopening, and moderator lifecycle powers stay distinct — WS8bm.
- browsing does not join and stale threads show as closed without writes — WS8bm.
- explicitly closing an already-stale thread persists closed_at — WS8bm.
- reopening a time-stale thread restarts its archive clock instead of leaving it closed — WS8bm.
- unlocking a time-stale thread reopens it instead of leaving it closed — WS8bm.
- posting to a thread persists closed_at for its stale siblings — WS8bm.
- thread index costs a constant number of queries as threads grow — WS8bm.
- joining accepts only thread notification preferences and preserves an existing preference when omitted — WS8bm.
- closed state contains locked threads and direct rooms reject thread creation — WS8bm.
- closed listing finds stale threads in SQL with one threads query — WS8bm.
- a deleted starter is represented explicitly so open clients clear its preview — WS8bm.
- content anchors only a message in the requested thread — WS8bm.
- converts a thread to work, assigns an eligible owner, and keeps an audit trail — WS12 / WS11; WS8bm HTTP seam.
- work owner must be an eligible parent-room member and a revoked owner stays visible as unavailable — WS12 / WS11; WS8bm HTTP seam.
- assigned owner can change work status but cannot reassign it — WS12 / WS11; WS8bm HTTP seam.
- only a thread manager can remove work tracking — WS12 / WS11; WS8bm HTTP seam.
- the work model also protects conversion when the owner field is omitted — WS12 / WS11; WS8bm HTTP seam.
- work status updates from separate stale instances produce one event per real change — WS12 / WS11; WS8bm HTTP seam.
- a manager can assign an eligible agent and the agent is notified — WS12 / WS11; WS8bm HTTP seam.
- the owner picker lists eligible agents with profiles and excludes ineligible ones — WS12 / WS11; WS8bm HTTP seam.
- a member who cannot manage the thread cannot assign an agent — WS12 / WS11; WS8bm HTTP seam.
- ordinary thread fields remain separate from work tracking — WS12 / WS11; WS8bm HTTP seam.

## test/controllers/channel_thread_messages_controller_test.rb

12 named declarations; case-level Rust attribution/signoff pending. Reference execution counts are in the main report.

- a post joins and reopens an unlocked closed thread atomically — WS8bm.
- locked threads block every edit and post while delete stays author-or-admin — WS8bm.
- thread unread state changes for every joined user regardless of preference — WS8bm.
- editing a thread message to add a post URL broadcasts the new card — WS15g / WS15e; WS8bm render integration.
- editing a thread message to remove a post URL broadcasts an empty card container — WS15g / WS15e; WS8bm render integration.
- destroy broadcasts tombstone updates and a thread summary refresh — WS8bm.
- edited thread messages show an edited marker — WS8bm.
- editing a thread message broadcasts its meta with the edited marker — WS8bm.
- identical thread message saves do not mark the message edited — WS8bm.
- nested HTML message URL redirects into the parent room shell — WS8b-r; WS8bm list integration.
- retried post with the same client id returns the original message — WS8bm. Attributed to `messages::review_tests::scalar_retry_paths_match_rails_bytes_and_rows`: complete responses/rows for true, false, numeric, string and blank IDs; actual pinned Rails oracle.
- thread system notes cannot be edited or deleted — WS8bm.

## test/controllers/channel_thread_messages_drive_attachments_test.rb

13 named declarations; case-level Rust attribution/signoff pending. Reference execution counts are in the main report.

- create with drive_file_ids stores them and reports them in JSON — WS8bm.
- create with attachments and no text is valid — WS8bm.
- create with an invalid id answers 422 and creates nothing — WS8bm.
- create with a scalar drive_file_ids answers 422 and creates nothing — WS8bm.
- create with more than 10 attachments answers 422 — WS8bm.
- update with a new set replaces the stored set — WS8bm.
- update without the key leaves the set alone — WS8bm.
- update with only the blank sentinel removes all attachments — WS8bm.
- update with an invalid id answers 422 and keeps the stored set — WS8bm.
- update with a scalar drive_file_ids answers 422 and keeps the stored set — WS8bm.
- a non-creator cannot change thread attachments — WS8bm.
- update with a submitted set broadcasts the attachments block over the thread stream — WS8bm.
- update without the key does not broadcast the attachments block — WS8bm.

## test/controllers/message_forwards_controller_test.rb

7 named declarations; case-level Rust attribution/signoff pending. Reference execution counts are in the main report.

- destinations returns reachable rooms and unlocked threads without caching — WS8bm.
- nested destination endpoint supports a thread message — WS8bm.
- destinations excludes board rooms — WS12 / WS11; WS8bm HTTP seam.
- create refuses board destinations on the server — WS12 / WS11; WS8bm HTTP seam.
- direct destinations use the other participant's display name — WS8bm.
- destinations show stale threads as closed without writing — WS8bm.
- destinations cost a constant number of queries as reachable rooms grow — WS8bm.

## test/controllers/message_forward_sources_controller_test.rb

2 named declarations; case-level Rust attribution/signoff pending. Reference execution counts are in the main report.

- source endpoint is no-store and keeps inaccessible source identity out of the response — WS8bm.
- source endpoint returns only the canonical URL to a viewer who still has access — WS8bm.
