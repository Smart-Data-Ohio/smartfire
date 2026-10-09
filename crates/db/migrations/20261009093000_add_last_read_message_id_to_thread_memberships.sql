-- How far each member has read a thread: the newest message id when they last read it. A thread
-- ping counts toward the sidebar's red count only while the thread is unread and the ping is
-- newer than this, so a reply after a read doesn't bring back the mentions already read.
-- Additive: a nullable column (null counts every unread ping, as before). Threads already read
-- count as read up to now, so nothing reappears after deploy; unread threads keep counting what
-- they count today. Plus a partial index for the counts (below). No jobs, no triggers.
ALTER TABLE "thread_memberships" ADD COLUMN "last_read_message_id" integer;
UPDATE "thread_memberships" SET "last_read_message_id" = (SELECT MAX("id") FROM "messages") WHERE "unread_at" IS NULL;
-- The sidebar counts read the viewer's unread message pings (mentions, replies, thread activity)
-- on every publish; this partial index holds just those, so they're found without walking the
-- viewer's whole inbox history.
CREATE INDEX "index_activity_items_on_unread_message_pings" ON "activity_items" ("user_id", "event_type") WHERE read_at IS NULL AND source_type = 'Message';
