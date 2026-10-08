-- The SPA's retry identity for a board post, so a retried creation finds the post the first
-- attempt made, across restarts. Additive: a nullable column and a partial unique index.
ALTER TABLE "channel_threads" ADD COLUMN "client_post_id" varchar;
CREATE UNIQUE INDEX "index_channel_threads_on_room_creator_client_post_id" ON "channel_threads" ("room_id", "creator_id", "client_post_id") WHERE client_post_id IS NOT NULL;
