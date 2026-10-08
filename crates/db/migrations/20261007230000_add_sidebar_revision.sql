-- The sidebar revision: a counter bumped in the same write transaction as any change to what a
-- sidebar row is made from (crates/api/src/dto.rs: rooms, memberships, messages, activity_items,
-- thread_memberships, and users' names for direct rooms), so a row read in one snapshot with the
-- counter carries a revision that follows commit order. Keep these triggers in sync with
-- dto::sidebar_row and dto::notification_counts. Columns that never reach a row (presence
-- heartbeats, streaming progress, message bodies) don't bump it.
-- One row, created by the first bump (a migration can't leave rows behind). The insert can't
-- conflict, so an outer statement's OR REPLACE / OR IGNORE can't turn it into a reset.
CREATE TABLE "sidebar_revisions" ("id" integer PRIMARY KEY NOT NULL CHECK ("id" = 1), "value" integer NOT NULL);

CREATE TRIGGER "sidebar_revision_rooms_insert"
AFTER INSERT ON "rooms"
BEGIN
    INSERT INTO "sidebar_revisions" ("id", "value") SELECT 1, 0 WHERE NOT EXISTS (SELECT 1 FROM "sidebar_revisions");
    UPDATE "sidebar_revisions" SET "value" = "value" + 1 WHERE "id" = 1;
END;

CREATE TRIGGER "sidebar_revision_rooms_delete"
AFTER DELETE ON "rooms"
BEGIN
    INSERT INTO "sidebar_revisions" ("id", "value") SELECT 1, 0 WHERE NOT EXISTS (SELECT 1 FROM "sidebar_revisions");
    UPDATE "sidebar_revisions" SET "value" = "value" + 1 WHERE "id" = 1;
END;

CREATE TRIGGER "sidebar_revision_rooms_update"
AFTER UPDATE OF "id", "type", "name", "icon_name", "creator_id", "created_at", "updated_at", "deleted_at" ON "rooms"
WHEN OLD."id" IS NOT NEW."id" OR OLD."type" IS NOT NEW."type" OR OLD."name" IS NOT NEW."name" OR OLD."icon_name" IS NOT NEW."icon_name" OR OLD."creator_id" IS NOT NEW."creator_id" OR OLD."created_at" IS NOT NEW."created_at" OR OLD."updated_at" IS NOT NEW."updated_at" OR OLD."deleted_at" IS NOT NEW."deleted_at"
BEGIN
    INSERT INTO "sidebar_revisions" ("id", "value") SELECT 1, 0 WHERE NOT EXISTS (SELECT 1 FROM "sidebar_revisions");
    UPDATE "sidebar_revisions" SET "value" = "value" + 1 WHERE "id" = 1;
END;

CREATE TRIGGER "sidebar_revision_memberships_insert"
AFTER INSERT ON "memberships"
BEGIN
    INSERT INTO "sidebar_revisions" ("id", "value") SELECT 1, 0 WHERE NOT EXISTS (SELECT 1 FROM "sidebar_revisions");
    UPDATE "sidebar_revisions" SET "value" = "value" + 1 WHERE "id" = 1;
END;

CREATE TRIGGER "sidebar_revision_memberships_delete"
AFTER DELETE ON "memberships"
BEGIN
    INSERT INTO "sidebar_revisions" ("id", "value") SELECT 1, 0 WHERE NOT EXISTS (SELECT 1 FROM "sidebar_revisions");
    UPDATE "sidebar_revisions" SET "value" = "value" + 1 WHERE "id" = 1;
END;

CREATE TRIGGER "sidebar_revision_memberships_update"
AFTER UPDATE OF "id", "room_id", "user_id", "involvement", "unread_at", "last_read_message_id", "room_category_id", "favorite_position", "stage_role" ON "memberships"
WHEN OLD."id" IS NOT NEW."id" OR OLD."room_id" IS NOT NEW."room_id" OR OLD."user_id" IS NOT NEW."user_id" OR OLD."involvement" IS NOT NEW."involvement" OR OLD."unread_at" IS NOT NEW."unread_at" OR OLD."last_read_message_id" IS NOT NEW."last_read_message_id" OR OLD."room_category_id" IS NOT NEW."room_category_id" OR OLD."favorite_position" IS NOT NEW."favorite_position" OR OLD."stage_role" IS NOT NEW."stage_role"
BEGIN
    INSERT INTO "sidebar_revisions" ("id", "value") SELECT 1, 0 WHERE NOT EXISTS (SELECT 1 FROM "sidebar_revisions");
    UPDATE "sidebar_revisions" SET "value" = "value" + 1 WHERE "id" = 1;
END;

CREATE TRIGGER "sidebar_revision_messages_insert"
AFTER INSERT ON "messages"
BEGIN
    INSERT INTO "sidebar_revisions" ("id", "value") SELECT 1, 0 WHERE NOT EXISTS (SELECT 1 FROM "sidebar_revisions");
    UPDATE "sidebar_revisions" SET "value" = "value" + 1 WHERE "id" = 1;
END;

CREATE TRIGGER "sidebar_revision_messages_delete"
AFTER DELETE ON "messages"
BEGIN
    INSERT INTO "sidebar_revisions" ("id", "value") SELECT 1, 0 WHERE NOT EXISTS (SELECT 1 FROM "sidebar_revisions");
    UPDATE "sidebar_revisions" SET "value" = "value" + 1 WHERE "id" = 1;
END;

CREATE TRIGGER "sidebar_revision_messages_update"
AFTER UPDATE OF "id", "room_id", "thread_id", "created_at", "creator_id", "system_note" ON "messages"
WHEN OLD."id" IS NOT NEW."id" OR OLD."room_id" IS NOT NEW."room_id" OR OLD."thread_id" IS NOT NEW."thread_id" OR OLD."created_at" IS NOT NEW."created_at" OR OLD."creator_id" IS NOT NEW."creator_id" OR OLD."system_note" IS NOT NEW."system_note"
BEGIN
    INSERT INTO "sidebar_revisions" ("id", "value") SELECT 1, 0 WHERE NOT EXISTS (SELECT 1 FROM "sidebar_revisions");
    UPDATE "sidebar_revisions" SET "value" = "value" + 1 WHERE "id" = 1;
END;

CREATE TRIGGER "sidebar_revision_activity_items_insert"
AFTER INSERT ON "activity_items"
BEGIN
    INSERT INTO "sidebar_revisions" ("id", "value") SELECT 1, 0 WHERE NOT EXISTS (SELECT 1 FROM "sidebar_revisions");
    UPDATE "sidebar_revisions" SET "value" = "value" + 1 WHERE "id" = 1;
END;

CREATE TRIGGER "sidebar_revision_activity_items_delete"
AFTER DELETE ON "activity_items"
BEGIN
    INSERT INTO "sidebar_revisions" ("id", "value") SELECT 1, 0 WHERE NOT EXISTS (SELECT 1 FROM "sidebar_revisions");
    UPDATE "sidebar_revisions" SET "value" = "value" + 1 WHERE "id" = 1;
END;

CREATE TRIGGER "sidebar_revision_activity_items_update"
AFTER UPDATE OF "id", "user_id", "source_type", "source_id", "event_type", "read_at" ON "activity_items"
WHEN OLD."id" IS NOT NEW."id" OR OLD."user_id" IS NOT NEW."user_id" OR OLD."source_type" IS NOT NEW."source_type" OR OLD."source_id" IS NOT NEW."source_id" OR OLD."event_type" IS NOT NEW."event_type" OR OLD."read_at" IS NOT NEW."read_at"
BEGIN
    INSERT INTO "sidebar_revisions" ("id", "value") SELECT 1, 0 WHERE NOT EXISTS (SELECT 1 FROM "sidebar_revisions");
    UPDATE "sidebar_revisions" SET "value" = "value" + 1 WHERE "id" = 1;
END;

CREATE TRIGGER "sidebar_revision_thread_memberships_insert"
AFTER INSERT ON "thread_memberships"
BEGIN
    INSERT INTO "sidebar_revisions" ("id", "value") SELECT 1, 0 WHERE NOT EXISTS (SELECT 1 FROM "sidebar_revisions");
    UPDATE "sidebar_revisions" SET "value" = "value" + 1 WHERE "id" = 1;
END;

CREATE TRIGGER "sidebar_revision_thread_memberships_delete"
AFTER DELETE ON "thread_memberships"
BEGIN
    INSERT INTO "sidebar_revisions" ("id", "value") SELECT 1, 0 WHERE NOT EXISTS (SELECT 1 FROM "sidebar_revisions");
    UPDATE "sidebar_revisions" SET "value" = "value" + 1 WHERE "id" = 1;
END;

CREATE TRIGGER "sidebar_revision_thread_memberships_update"
AFTER UPDATE OF "id", "thread_id", "user_id", "unread_at" ON "thread_memberships"
WHEN OLD."id" IS NOT NEW."id" OR OLD."thread_id" IS NOT NEW."thread_id" OR OLD."user_id" IS NOT NEW."user_id" OR OLD."unread_at" IS NOT NEW."unread_at"
BEGIN
    INSERT INTO "sidebar_revisions" ("id", "value") SELECT 1, 0 WHERE NOT EXISTS (SELECT 1 FROM "sidebar_revisions");
    UPDATE "sidebar_revisions" SET "value" = "value" + 1 WHERE "id" = 1;
END;

CREATE TRIGGER "sidebar_revision_users_update"
AFTER UPDATE OF "name" ON "users"
WHEN OLD."name" IS NOT NEW."name"
BEGIN
    INSERT INTO "sidebar_revisions" ("id", "value") SELECT 1, 0 WHERE NOT EXISTS (SELECT 1 FROM "sidebar_revisions");
    UPDATE "sidebar_revisions" SET "value" = "value" + 1 WHERE "id" = 1;
END;
