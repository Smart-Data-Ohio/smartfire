# WS8bm named Rails controller case inventory

Pinned reference: `d7c7de92`. This is a case attribution backlog, not a claim that the cases are unimplemented. The report records the independently executed Rust aggregate tests and Rails reference counts separately. All names below still require explicit case-level Rust attribution/signoff; no one-to-one port count is claimed. Browser/system execution remains deferred.

## test/controllers/messages_controller_test.rb

56 named declarations; case-level Rust attribution/signoff pending. Reference execution counts are in the main report.

- index returns the last page by default — WS8bm.
- index is not found for a soft-deleted room — WS8bm.
- index returns a page before the specified message — WS8bm.
- index returns a page after the specified message — WS8bm.
- index returns no_content when there are no messages — WS8bm.
- index etag changes when an off-page reply source is edited — WS8bm.
- index etag changes when a card fetch completes — WS15g / WS15e; WS8bm render integration.
- index etag changes when an author is renamed — WS8bm.
- index etag changes when the older of two pins is removed — WS8b-m2; WS8bm cache integration.
- get renders a single message belonging to the user — WS8bm.
- room message list announces live appends — WS8b-r; WS8bm list integration.
- image attachments use the filename as alt text — WS8bm.
- creating a message broadcasts the message to the room — WS8bm.
- broadcast message actions preserve a nonstandard request port — WS8bm.
- creating a Markdown message preserves its source and derives the rich body — WS8bm.
- preview renders the same safe Markdown without writing — WS8bm.
- preview requires room membership — WS8bm.
- preview rejects oversized Markdown without parsing or writing it — WS8bm.
- preview is protected against cross-site form submissions — WS8bm.
- creating a message broadcasts unread room to each member — WS8bm.
- creating a message doesn't broadcast unread room to non-members — WS8bm.
- update updates a message belonging to the user — WS8bm.
- updating a Markdown message preserves exact new source — WS8bm.
- a legacy body update clears stale Markdown mode — WS8bm.
- editing a message to add a PR URL broadcasts the new card — WS15g / WS15e; WS8bm render integration.
- editing a message to remove a PR URL broadcasts an empty card container — WS15g / WS15e; WS8bm render integration.
- legacy rich-text edits re-sync card references — WS15g / WS15e; WS8bm render integration.
- messages render empty card containers for future broadcasts — WS15g / WS15e; WS8bm render integration.
- admin cannot update a message belonging to another user — WS8bm.
- destroy destroys a message belonging to the user — WS8bm.
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
- ensure non-admin can't destroy a message belonging to another user — WS8bm.
- mentioning a bot triggers a webhook — WS11; WS8bm HTTP integration.
- mentioning a bot from Markdown triggers a webhook — WS11; WS8bm HTTP integration.
- mentioning an agent-backed bot skips the legacy webhook job — WS11; WS8bm HTTP integration.
- mentioning an agent-backed bot posts exactly one webhook with the agent key — WS11; WS8bm HTTP integration.
- revoked agent delivery posts no webhook — WS11; WS8bm HTTP integration.
- mentioning a bot without an agent row still uses the legacy webhook — WS11; WS8bm HTTP integration.
- retried create with the same client id returns the original message — WS8bm.
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

17 named declarations; case-level Rust attribution/signoff pending. Reference execution counts are in the main report.

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
- the boost action opens the soft keyboard — WS12 / WS11; WS8bm HTTP seam.
- action metadata groups reaction counts by distinct reactor — WS8bm.
- the reaction tooltip lists reactors as plain text, never interactive content — WS8bm.

## test/controllers/channel_threads_controller_test.rb

24 named declarations; case-level Rust attribution/signoff pending. Reference execution counts are in the main report.

- creation accepts nested thread message parameters and joins only the creator — WS8bm.
- retried creation with the same first-message client id returns the existing thread — WS8bm.
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
- retried post with the same client id returns the original message — WS8bm.
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
