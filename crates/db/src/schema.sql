CREATE TABLE "accounts" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "custom_styles" text, "join_code" varchar NOT NULL, "name" varchar NOT NULL, "settings" json, "singleton_guard" integer DEFAULT 0 NOT NULL, "updated_at" datetime(6) NOT NULL);
CREATE UNIQUE INDEX "index_accounts_on_singleton_guard" ON "accounts" ("singleton_guard");
CREATE TABLE "action_mailbox_inbound_emails" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "message_checksum" varchar NOT NULL, "message_id" varchar NOT NULL, "status" integer DEFAULT 0 NOT NULL, "updated_at" datetime(6) NOT NULL);
CREATE UNIQUE INDEX "index_action_mailbox_inbound_emails_uniqueness" ON "action_mailbox_inbound_emails" ("message_id", "message_checksum");
CREATE TABLE "action_text_rich_texts" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "body" text, "created_at" datetime(6) NOT NULL, "name" varchar NOT NULL, "record_id" bigint NOT NULL, "record_type" varchar NOT NULL, "updated_at" datetime(6) NOT NULL);
CREATE UNIQUE INDEX "index_action_text_rich_texts_uniqueness" ON "action_text_rich_texts" ("record_type", "record_id", "name");
CREATE TABLE "active_storage_blobs" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "byte_size" bigint NOT NULL, "checksum" varchar, "content_type" varchar, "created_at" datetime(6) NOT NULL, "filename" varchar NOT NULL, "key" varchar NOT NULL, "message_processing_expires_at" datetime(6), "message_processing_token" varchar, "metadata" text, "service_name" varchar NOT NULL);
CREATE UNIQUE INDEX "index_active_storage_blobs_on_key" ON "active_storage_blobs" ("key");
CREATE TABLE "agent_approvals" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "action" varchar NOT NULL, "agent_credential_id" integer, "agent_id" integer NOT NULL, "created_at" datetime(6) NOT NULL, "decided_at" datetime(6), "decided_by_id" integer, "decision_note" varchar, "expires_at" datetime(6) NOT NULL, "external_id" varchar, "fizzy_connected_account_id" integer, "fizzy_user_id" varchar, "fizzy_user_name" varchar, "github_account_id" integer, "github_login" varchar, "payload" text, "room_id" integer, "status" varchar DEFAULT 'pending' NOT NULL, "summary" text NOT NULL, "updated_at" datetime(6) NOT NULL);
CREATE UNIQUE INDEX "index_agent_approvals_on_agent_id_and_external_id" ON "agent_approvals" ("agent_id", "external_id") WHERE external_id IS NOT NULL;
CREATE INDEX "index_agent_approvals_on_agent_id_and_status" ON "agent_approvals" ("agent_id", "status");
CREATE INDEX "index_agent_approvals_on_fizzy_connected_account_id" ON "agent_approvals" ("fizzy_connected_account_id");
CREATE TABLE "agent_budget_notices" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "agent_id" integer NOT NULL, "cap" varchar NOT NULL, "created_at" datetime(6) NOT NULL, "day" date NOT NULL, "updated_at" datetime(6) NOT NULL);
CREATE UNIQUE INDEX "index_agent_budget_notices_on_agent_cap_day" ON "agent_budget_notices" ("agent_id", "cap", "day");
CREATE TABLE "agent_credentials" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "agent_id" integer NOT NULL, "created_at" datetime(6) NOT NULL, "created_by_id" integer NOT NULL, "expires_at" datetime(6), "last_used_at" datetime(6), "last_used_ip" varchar, "name" varchar NOT NULL, "revoked_at" datetime(6), "token_digest" varchar NOT NULL, "token_last_four" varchar NOT NULL, "updated_at" datetime(6) NOT NULL);
CREATE INDEX "index_agent_credentials_on_agent_id_and_revoked_at" ON "agent_credentials" ("agent_id", "revoked_at");
CREATE UNIQUE INDEX "index_agent_credentials_on_token_digest" ON "agent_credentials" ("token_digest");
CREATE TABLE "agent_events" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "actor_id" integer, "agent_approval_id" integer, "agent_credential_id" integer, "agent_id" integer NOT NULL, "chain_id" varchar, "created_at" datetime(6) NOT NULL, "detail" varchar, "event_type" varchar NOT NULL, "hop" integer DEFAULT 0 NOT NULL, "message_id" integer, "metadata" json, "outcome" varchar, "room_id" integer, "webhook_attempts" integer DEFAULT 0 NOT NULL, "webhook_last_error" text, "webhook_next_attempt_at" datetime(6), "webhook_status" varchar DEFAULT 'none' NOT NULL);
CREATE UNIQUE INDEX "index_agent_events_on_agent_fizzy_approval" ON "agent_events" ("agent_id", "agent_approval_id") WHERE event_type = 'fizzy_action_completed' AND agent_approval_id IS NOT NULL;
CREATE UNIQUE INDEX "index_agent_events_on_agent_github_approval" ON "agent_events" ("agent_id", "agent_approval_id") WHERE event_type = 'github_action_completed' AND agent_approval_id IS NOT NULL;
CREATE INDEX "index_agent_events_on_agent_id_and_created_at" ON "agent_events" ("agent_id", "created_at");
CREATE INDEX "index_agent_events_on_agent_outcome_id" ON "agent_events" ("agent_id", "outcome", "id");
CREATE INDEX "index_agent_events_on_webhook_recovery" ON "agent_events" ("webhook_status", "webhook_next_attempt_at");
CREATE TABLE "agent_grants" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "agent_id" integer NOT NULL, "capability" varchar NOT NULL, "created_at" datetime(6) NOT NULL, "granted_by_id" integer NOT NULL, "revoked_at" datetime(6), "room_id" integer, "updated_at" datetime(6) NOT NULL);
CREATE UNIQUE INDEX "index_agent_grants_on_agent_capability_active_workspace" ON "agent_grants" ("agent_id", "capability") WHERE revoked_at IS NULL AND room_id IS NULL;
CREATE INDEX "index_agent_grants_on_agent_id_and_revoked_at" ON "agent_grants" ("agent_id", "revoked_at");
CREATE UNIQUE INDEX "index_agent_grants_on_agent_room_capability_active" ON "agent_grants" ("agent_id", "room_id", "capability") WHERE revoked_at IS NULL AND room_id IS NOT NULL;
CREATE TABLE "agent_steps" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "agent_id" integer NOT NULL, "channel_thread_id" integer, "created_at" datetime(6) NOT NULL, "duration_ms" integer, "input_summary" text, "message_id" integer, "name" varchar NOT NULL, "output_summary" text, "position" integer DEFAULT 0 NOT NULL, "status" varchar DEFAULT 'running' NOT NULL, "updated_at" datetime(6) NOT NULL);
CREATE INDEX "index_agent_steps_on_agent_id" ON "agent_steps" ("agent_id");
CREATE INDEX "index_agent_steps_on_channel_thread_id" ON "agent_steps" ("channel_thread_id");
CREATE INDEX "index_agent_steps_on_message_id" ON "agent_steps" ("message_id");
CREATE TABLE "agents" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "daily_board_post_cap" integer, "daily_external_action_cap" integer, "daily_message_cap" integer, "description" text, "kind" varchar DEFAULT 'personal' NOT NULL, "last_seen_at" datetime(6), "owner_id" integer, "provider" varchar, "runtime" varchar, "status" varchar DEFAULT 'idle' NOT NULL, "status_changed_at" datetime(6), "status_note" varchar, "suspended_at" datetime(6), "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, "webhook_signing_secret" varchar, "working_presence" varchar, "working_presence_expires_at" datetime(6));
CREATE INDEX "index_agents_on_owner_id_and_kind" ON "agents" ("owner_id", "kind");
CREATE UNIQUE INDEX "index_agents_on_user_id" ON "agents" ("user_id");
CREATE TABLE "audit_logs" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "action" varchar NOT NULL, "actor_id" bigint, "actor_label" varchar, "created_at" datetime(6) NOT NULL, "details" json, "ip_address" varchar, "target_id" bigint, "target_label" varchar, "target_type" varchar, "updated_at" datetime(6) NOT NULL, "user_agent" varchar);
CREATE INDEX "index_audit_logs_on_action_and_ip_address_and_created_at" ON "audit_logs" ("action", "ip_address", "created_at");
CREATE INDEX "index_audit_logs_on_action" ON "audit_logs" ("action");
CREATE INDEX "index_audit_logs_on_actor_id" ON "audit_logs" ("actor_id");
CREATE INDEX "index_audit_logs_on_created_at" ON "audit_logs" ("created_at");
CREATE INDEX "index_audit_logs_on_target_type_and_target_id" ON "audit_logs" ("target_type", "target_id");
CREATE TABLE "background_jobs" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "arguments" json NOT NULL, "attempts" integer DEFAULT 0 NOT NULL, "claimed_by" varchar, "created_at" datetime(6) NOT NULL, "failed_at" datetime(6), "job_class" varchar NOT NULL, "last_error" text, "lease_expires_at" datetime(6), "payload_version" integer DEFAULT 1 NOT NULL, "queue_name" varchar NOT NULL, "run_at" datetime(6) NOT NULL, "status" varchar DEFAULT 'ready' NOT NULL, "updated_at" datetime(6) NOT NULL);
CREATE INDEX "index_background_jobs_for_claiming" ON "background_jobs" ("status", "queue_name", "run_at");
CREATE TABLE "event_attendances" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "event_id" integer NOT NULL, "response" varchar NOT NULL, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL);
CREATE UNIQUE INDEX "index_event_attendances_on_event_id_and_user_id" ON "event_attendances" ("event_id", "user_id");
CREATE TABLE "events" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "cancelled_at" datetime(6), "created_at" datetime(6) NOT NULL, "description" text, "ends_at" datetime(6), "meet_link" varchar, "meet_link_requested" boolean DEFAULT FALSE NOT NULL, "organizer_id" integer NOT NULL, "recurrence_rule" varchar, "recurrence_until" date, "reminded_at" datetime(6), "room_id" integer NOT NULL, "series_id" integer, "starts_at" datetime(6) NOT NULL, "time_zone" varchar NOT NULL, "title" varchar NOT NULL, "updated_at" datetime(6) NOT NULL, "venue_room_id" integer);
CREATE INDEX "index_events_on_organizer_id" ON "events" ("organizer_id");
CREATE INDEX "index_events_on_reminded_starts" ON "events" ("reminded_at", "starts_at");
CREATE INDEX "index_events_on_room_id_and_starts_at" ON "events" ("room_id", "starts_at");
CREATE UNIQUE INDEX "index_events_on_series_slot" ON "events" ("series_id", "starts_at") WHERE series_id IS NOT NULL AND cancelled_at IS NULL;
CREATE INDEX "index_events_on_series_id" ON "events" ("series_id");
CREATE INDEX "index_events_on_venue_room_id" ON "events" ("venue_room_id");
CREATE TABLE "fizzy_cards" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "account_id" varchar NOT NULL, "created_at" datetime(6) NOT NULL, "number" integer NOT NULL, "updated_at" datetime(6) NOT NULL);
CREATE UNIQUE INDEX "index_fizzy_cards_on_account_id_and_number" ON "fizzy_cards" ("account_id", "number");
CREATE TABLE "github_pull_requests" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "author_avatar_url" varchar, "author_login" varchar, "base_branch" varchar, "changed_files" text, "changed_files_fetched_at" datetime(6), "check_status" varchar, "created_at" datetime(6) NOT NULL, "fetch_error" varchar, "fetch_requested_at" datetime(6), "fetched_at" datetime(6), "github_updated_at" datetime(6), "head_branch" varchar, "head_sha" varchar, "html_url" varchar, "number" integer NOT NULL, "owner" varchar NOT NULL, "payload" json, "private" boolean, "repo" varchar NOT NULL, "review_decision" varchar, "state" varchar, "title" varchar, "updated_at" datetime(6) NOT NULL);
CREATE UNIQUE INDEX "index_github_pull_requests_on_owner_repo_number" ON "github_pull_requests" ("owner", "repo", "number");
CREATE TABLE "github_webhook_deliveries" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "delivery_guid" varchar NOT NULL, "event" varchar, "updated_at" datetime(6) NOT NULL);
CREATE UNIQUE INDEX "index_github_webhook_deliveries_on_delivery_guid" ON "github_webhook_deliveries" ("delivery_guid");
CREATE TABLE "huddle_cleanups" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "attempts" integer DEFAULT 0 NOT NULL, "completed_at" datetime(6), "created_at" datetime(6) NOT NULL, "enqueued_at" datetime(6), "huddle_grant_id" integer, "identity" varchar, "last_attempted_at" datetime(6), "next_attempt_at" datetime(6), "operation" varchar NOT NULL, "room_name" varchar NOT NULL, "updated_at" datetime(6) NOT NULL);
CREATE INDEX "index_huddle_cleanups_on_completed_at_and_next_attempt_at" ON "huddle_cleanups" ("completed_at", "next_attempt_at");
CREATE INDEX "index_huddle_cleanups_on_huddle_grant_id" ON "huddle_cleanups" ("huddle_grant_id");
CREATE UNIQUE INDEX "index_huddle_cleanups_on_unique_participant_removal" ON "huddle_cleanups" ("operation", "huddle_grant_id") WHERE operation = 'remove_participant';
CREATE UNIQUE INDEX "index_huddle_cleanups_on_unique_room_deletion" ON "huddle_cleanups" ("operation", "room_name") WHERE operation = 'delete_room';
CREATE TABLE "huddle_grants" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "identity" varchar NOT NULL, "last_issued_at" datetime(6), "last_seen_at" datetime(6), "membership_id" integer NOT NULL, "revoked_at" datetime(6), "room_id" integer NOT NULL, "room_name" varchar NOT NULL, "server_muted" boolean DEFAULT FALSE NOT NULL, "session_id" integer NOT NULL, "stage_role" varchar, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL);
CREATE UNIQUE INDEX "index_huddle_grants_on_identity" ON "huddle_grants" ("identity");
CREATE INDEX "index_huddle_grants_on_membership_id" ON "huddle_grants" ("membership_id");
CREATE INDEX "index_huddle_grants_on_room_and_last_seen_at" ON "huddle_grants" ("room_id", "last_seen_at");
CREATE INDEX "index_huddle_grants_on_room_id" ON "huddle_grants" ("room_id");
CREATE UNIQUE INDEX "index_active_huddle_grants_on_session_and_membership" ON "huddle_grants" ("session_id", "membership_id") WHERE revoked_at IS NULL;
CREATE INDEX "index_huddle_grants_on_session_id" ON "huddle_grants" ("session_id");
CREATE INDEX "index_huddle_grants_on_user_id" ON "huddle_grants" ("user_id");
CREATE TABLE "link_embeds" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "description" text, "expires_at" datetime(6), "fetch_error" varchar, "fetch_requested_at" datetime(6), "fetched_at" datetime(6), "image_url" varchar, "normalized_url" varchar NOT NULL, "site_name" varchar, "title" varchar, "updated_at" datetime(6) NOT NULL);
CREATE UNIQUE INDEX "index_link_embeds_on_normalized_url" ON "link_embeds" ("normalized_url");
CREATE TABLE "memberships" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "connected_at" datetime(6), "connections" integer DEFAULT 0 NOT NULL, "created_at" datetime(6) NOT NULL, "favorite_position" integer, "hand_raised_at" datetime(6), "involvement" varchar DEFAULT 'mentions', "last_huddle_join_push_at" datetime(6), "last_read_message_id" bigint, "room_category_id" bigint, "room_id" integer NOT NULL, "server_muted_at" datetime(6), "stage_role" varchar, "unread_at" datetime(6), "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL);
CREATE INDEX "index_memberships_on_last_read_message_id" ON "memberships" ("last_read_message_id");
CREATE INDEX "index_memberships_on_room_id_and_created_at" ON "memberships" ("room_id", "created_at");
CREATE INDEX "index_memberships_on_room_id_and_stage_role" ON "memberships" ("room_id", "stage_role");
CREATE UNIQUE INDEX "index_memberships_on_room_id_and_user_id" ON "memberships" ("room_id", "user_id");
CREATE INDEX "index_memberships_on_room_id" ON "memberships" ("room_id");
CREATE INDEX "index_memberships_on_user_and_favorite" ON "memberships" ("user_id", "favorite_position");
CREATE INDEX "index_memberships_on_user_and_category" ON "memberships" ("user_id", "room_category_id");
CREATE INDEX "index_memberships_on_user_id" ON "memberships" ("user_id");
CREATE TABLE "room_categories" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "collapsed" boolean DEFAULT FALSE NOT NULL, "created_at" datetime(6) NOT NULL, "name" varchar NOT NULL, "position" integer DEFAULT 0 NOT NULL, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL);
CREATE INDEX "index_room_categories_on_user_and_position" ON "room_categories" ("user_id", "position");
CREATE INDEX "index_room_categories_on_user_id" ON "room_categories" ("user_id");
CREATE TABLE "rooms" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "creator_id" bigint NOT NULL, "deleted_at" datetime(6), "destroy_enqueued_at" datetime(6), "direct_member_key" varchar, "icon_name" varchar, "inbound_email_token" varchar, "name" varchar, "pins_changed_at" datetime(6), "type" varchar NOT NULL, "updated_at" datetime(6) NOT NULL, "client_room_id" varchar);
CREATE UNIQUE INDEX "index_rooms_on_direct_member_key" ON "rooms" ("direct_member_key") WHERE direct_member_key IS NOT NULL AND deleted_at IS NULL;
CREATE UNIQUE INDEX "index_rooms_on_inbound_email_token" ON "rooms" ("inbound_email_token");
CREATE TABLE "streams" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "ended_at" datetime(6), "membership_id" integer NOT NULL, "quality" varchar NOT NULL, "room_id" integer NOT NULL, "started_at" datetime(6) NOT NULL, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL);
CREATE INDEX "index_streams_on_membership_id" ON "streams" ("membership_id");
CREATE UNIQUE INDEX "index_streams_on_room_id" ON "streams" ("room_id") WHERE ended_at IS NULL;
CREATE TABLE "thread_tags" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "channel_thread_id" integer NOT NULL, "created_at" datetime(6) NOT NULL, "name" varchar NOT NULL, "updated_at" datetime(6) NOT NULL);
CREATE UNIQUE INDEX "index_thread_tags_on_channel_thread_id_and_name" ON "thread_tags" ("channel_thread_id", "name");
CREATE INDEX "index_thread_tags_on_name" ON "thread_tags" ("name");
CREATE TABLE "twitter_posts" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "author_avatar_url" varchar, "author_handle" varchar, "author_name" varchar, "created_at" datetime(6) NOT NULL, "fetch_error" varchar, "fetch_requested_at" datetime(6), "fetched_at" datetime(6), "likes" integer, "media" json, "post_id" varchar NOT NULL, "posted_at" datetime(6), "quote" json, "replies" integer, "reposts" integer, "text" text, "updated_at" datetime(6) NOT NULL, "url" varchar);
CREATE UNIQUE INDEX "index_twitter_posts_on_post_id" ON "twitter_posts" ("post_id");
CREATE TABLE "users" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "bio" text, "bot_token" varchar, "bot_token_digest" varchar, "created_at" datetime(6) NOT NULL, "custom_status_emoji" varchar, "custom_status_expires_at" datetime(6), "custom_status_text" varchar, "dnd_enabled" boolean DEFAULT FALSE NOT NULL, "dnd_until" datetime(6), "email_address" varchar, "email_self_changed_at" datetime(6), "github_login" varchar, "google_email_link_allowed" boolean DEFAULT FALSE NOT NULL, "icon_name" varchar, "inbox_preferences" json DEFAULT '{}', "meeting_dnd_enabled" boolean DEFAULT FALSE NOT NULL, "meeting_status_enabled" boolean DEFAULT FALSE NOT NULL, "name" varchar NOT NULL, "ooo_broadcast" boolean, "ooo_calendar_enabled" boolean DEFAULT FALSE NOT NULL, "ooo_note" varchar(140), "ooo_notify_enabled" boolean DEFAULT FALSE NOT NULL, "ooo_until" datetime(6), "password_digest" varchar, "presence_setting" varchar DEFAULT 'auto' NOT NULL, "push_to_talk_key" varchar, "quiet_hours_enabled" boolean DEFAULT FALSE NOT NULL, "quiet_hours_end_minute" integer, "quiet_hours_start_minute" integer, "role" integer DEFAULT 0 NOT NULL, "status" integer DEFAULT 0 NOT NULL, "text_size" varchar DEFAULT 'default' NOT NULL, "theme" varchar DEFAULT 'system' NOT NULL, "time_zone" varchar, "time_zone_explicit" boolean DEFAULT FALSE NOT NULL, "tour_completed_at" datetime(6), "updated_at" datetime(6) NOT NULL, "voice_mode" varchar, activity_revision INTEGER NOT NULL DEFAULT 0, appearance_preferences TEXT, pronouns TEXT, nickname TEXT);
CREATE UNIQUE INDEX "index_users_on_lower_github_login" ON "users" (LOWER(github_login)) WHERE github_login IS NOT NULL;
CREATE UNIQUE INDEX "index_users_on_bot_token" ON "users" ("bot_token");
CREATE UNIQUE INDEX "index_users_on_bot_token_digest" ON "users" ("bot_token_digest");
CREATE UNIQUE INDEX "index_users_on_email_address" ON "users" ("email_address");
CREATE TABLE "active_storage_attachments" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "blob_id" bigint NOT NULL, "created_at" datetime(6) NOT NULL, "name" varchar NOT NULL, "record_id" bigint NOT NULL, "record_type" varchar NOT NULL, CONSTRAINT "fk_rails_c3b3935057"
FOREIGN KEY ("blob_id")
  REFERENCES "active_storage_blobs" ("id")
);
CREATE INDEX "index_active_storage_attachments_on_blob_id" ON "active_storage_attachments" ("blob_id");
CREATE UNIQUE INDEX "index_active_storage_attachments_uniqueness" ON "active_storage_attachments" ("record_type", "record_id", "name", "blob_id");
CREATE TABLE "active_storage_variant_records" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "blob_id" bigint NOT NULL, "variation_digest" varchar NOT NULL, CONSTRAINT "fk_rails_993965df05"
FOREIGN KEY ("blob_id")
  REFERENCES "active_storage_blobs" ("id")
);
CREATE UNIQUE INDEX "index_active_storage_variant_records_uniqueness" ON "active_storage_variant_records" ("blob_id", "variation_digest");
CREATE TABLE "activity_items" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "event_type" varchar NOT NULL, "handled_at" datetime(6), "read_at" datetime(6), "source_id" integer NOT NULL, "source_type" varchar NOT NULL, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_b0b312b5a5"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
 ON DELETE CASCADE);
CREATE INDEX "index_activity_items_on_event_type_and_created_at" ON "activity_items" ("event_type", "created_at");
CREATE INDEX "index_activity_items_on_source" ON "activity_items" ("source_type", "source_id");
CREATE INDEX "index_activity_items_on_user_and_state" ON "activity_items" ("user_id", "read_at", "handled_at", "created_at");
CREATE UNIQUE INDEX "index_activity_items_on_user_and_source" ON "activity_items" ("user_id", "source_type", "source_id");
CREATE INDEX "index_activity_items_on_user_id_and_updated_at" ON "activity_items" ("user_id", "updated_at");
CREATE TABLE "agent_slash_commands" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "agent_id" integer NOT NULL, "created_at" datetime(6) NOT NULL, "description" varchar, "name" varchar NOT NULL, "room_id" integer NOT NULL, "takes_arguments" boolean DEFAULT TRUE NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_873314717b"
FOREIGN KEY ("agent_id")
  REFERENCES "agents" ("id")
, CONSTRAINT "fk_rails_e604b7b265"
FOREIGN KEY ("room_id")
  REFERENCES "rooms" ("id")
);
CREATE INDEX "index_agent_slash_commands_on_agent_id" ON "agent_slash_commands" ("agent_id");
CREATE UNIQUE INDEX "index_agent_slash_commands_on_room_id_and_name" ON "agent_slash_commands" ("room_id", "name");
CREATE INDEX "index_agent_slash_commands_on_room_id" ON "agent_slash_commands" ("room_id");
CREATE TABLE "bans" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "ip_address" varchar NOT NULL, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_070022cd76"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE INDEX "index_bans_on_ip_address" ON "bans" ("ip_address");
CREATE INDEX "index_bans_on_user_id" ON "bans" ("user_id");
CREATE TABLE "board_sla_nudges" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "channel_thread_id" integer NOT NULL, "created_at" datetime(6) NOT NULL, "recipient_id" integer NOT NULL, "room_id" integer NOT NULL, "stage" varchar NOT NULL, "status_entered_at" datetime(6) NOT NULL, "updated_at" datetime(6) NOT NULL, "work_status" varchar NOT NULL, CONSTRAINT "fk_rails_3845fff888"
FOREIGN KEY ("room_id")
  REFERENCES "rooms" ("id")
, CONSTRAINT "fk_rails_148462f2b1"
FOREIGN KEY ("channel_thread_id")
  REFERENCES "channel_threads" ("id")
, CONSTRAINT "fk_rails_e4d13221ea"
FOREIGN KEY ("recipient_id")
  REFERENCES "users" ("id")
);
CREATE UNIQUE INDEX "index_board_sla_nudges_on_claim" ON "board_sla_nudges" ("channel_thread_id", "work_status", "stage", "status_entered_at");
CREATE INDEX "index_board_sla_nudges_on_channel_thread_id" ON "board_sla_nudges" ("channel_thread_id");
CREATE INDEX "index_board_sla_nudges_on_recipient_id" ON "board_sla_nudges" ("recipient_id");
CREATE INDEX "index_board_sla_nudges_on_room_id" ON "board_sla_nudges" ("room_id");
CREATE TABLE "board_sla_rules" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "escalate_after_minutes" integer NOT NULL, "nudge_after_minutes" integer NOT NULL, "room_id" integer NOT NULL, "updated_at" datetime(6) NOT NULL, "work_status" varchar NOT NULL, CONSTRAINT "fk_rails_1be46b603d"
FOREIGN KEY ("room_id")
  REFERENCES "rooms" ("id")
);
CREATE UNIQUE INDEX "index_board_sla_rules_on_room_id_and_work_status" ON "board_sla_rules" ("room_id", "work_status");
CREATE INDEX "index_board_sla_rules_on_room_id" ON "board_sla_rules" ("room_id");
CREATE TABLE "board_stale_digests" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "digest_on" date NOT NULL, "message_id" integer, "room_id" integer NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_fa8fba8d64"
FOREIGN KEY ("message_id")
  REFERENCES "messages" ("id")
, CONSTRAINT "fk_rails_7076508035"
FOREIGN KEY ("room_id")
  REFERENCES "rooms" ("id")
);
CREATE INDEX "index_board_stale_digests_on_message_id" ON "board_stale_digests" ("message_id");
CREATE UNIQUE INDEX "index_board_stale_digests_on_room_id_and_digest_on" ON "board_stale_digests" ("room_id", "digest_on");
CREATE INDEX "index_board_stale_digests_on_room_id" ON "board_stale_digests" ("room_id");
CREATE TABLE "board_tag_assignments" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "assignee_id" integer NOT NULL, "created_at" datetime(6) NOT NULL, "created_by_id" integer NOT NULL, "room_id" integer NOT NULL, "tag" varchar NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_5a6075b5d0"
FOREIGN KEY ("assignee_id")
  REFERENCES "users" ("id")
, CONSTRAINT "fk_rails_4e0656e6be"
FOREIGN KEY ("room_id")
  REFERENCES "rooms" ("id")
, CONSTRAINT "fk_rails_1e24e3b2b3"
FOREIGN KEY ("created_by_id")
  REFERENCES "users" ("id")
);
CREATE INDEX "index_board_tag_assignments_on_assignee_id" ON "board_tag_assignments" ("assignee_id");
CREATE INDEX "index_board_tag_assignments_on_created_by_id" ON "board_tag_assignments" ("created_by_id");
CREATE UNIQUE INDEX "index_board_tag_assignments_on_room_id_and_tag" ON "board_tag_assignments" ("room_id", "tag");
CREATE INDEX "index_board_tag_assignments_on_room_id" ON "board_tag_assignments" ("room_id");
CREATE TABLE "boosts" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "booster_id" integer NOT NULL, "content" varchar(16) NOT NULL, "created_at" datetime(6) NOT NULL, "message_id" integer NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_3539c52d73"
FOREIGN KEY ("message_id")
  REFERENCES "messages" ("id")
);
CREATE INDEX "index_boosts_on_booster_id" ON "boosts" ("booster_id");
CREATE INDEX "index_boosts_on_message_id" ON "boosts" ("message_id");
CREATE TABLE "calendar_meeting_caches" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "busy_intervals" json DEFAULT '[]' NOT NULL, "created_at" datetime(6) NOT NULL, "fetch_error" varchar, "fetched_at" datetime(6), "in_meeting_broadcast" boolean, "ooo_intervals" json DEFAULT '[]' NOT NULL, "refresh_pending_at" datetime(6), "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_a243682e27"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
 ON DELETE CASCADE);
CREATE UNIQUE INDEX "index_calendar_meeting_caches_on_user_id" ON "calendar_meeting_caches" ("user_id");
CREATE TABLE "calendar_push_channels" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "channel_id" varchar NOT NULL, "created_at" datetime(6) NOT NULL, "expires_at" datetime(6), "last_error" varchar, "last_message_number" bigint DEFAULT 0 NOT NULL, "last_notification_at" datetime(6), "resource_id" varchar, "token_digest" varchar NOT NULL, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_41e6c2a643"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE UNIQUE INDEX "index_calendar_push_channels_on_channel_id" ON "calendar_push_channels" ("channel_id");
CREATE UNIQUE INDEX "index_calendar_push_channels_on_user_id" ON "calendar_push_channels" ("user_id");
CREATE TABLE "channel_threads" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "auto_archive_after_minutes" integer DEFAULT 4320 NOT NULL, "closed_at" datetime(6), "created_at" datetime(6) NOT NULL, "creator_id" integer NOT NULL, "last_activity_at" datetime(6) NOT NULL, "locked_at" datetime(6), "messages_count" integer DEFAULT 0 NOT NULL, "name" varchar NOT NULL, "parent_message_id" integer, "result_markdown" text, "result_updated_at" datetime(6), "result_updated_by_id" integer, "room_id" integer NOT NULL, "run_url" varchar, "updated_at" datetime(6) NOT NULL, "work_owner_id" integer, "work_status" varchar, "work_status_changed_at" datetime(6), "client_post_id" varchar, CONSTRAINT "fk_rails_f494e709ed"
FOREIGN KEY ("creator_id")
  REFERENCES "users" ("id")
, CONSTRAINT "fk_rails_bb889ed91c"
FOREIGN KEY ("parent_message_id")
  REFERENCES "messages" ("id")
 ON DELETE SET NULL, CONSTRAINT "fk_rails_f33ab22d2f"
FOREIGN KEY ("room_id")
  REFERENCES "rooms" ("id")
, CONSTRAINT "fk_rails_4f89623244"
FOREIGN KEY ("work_owner_id")
  REFERENCES "users" ("id")
 ON DELETE SET NULL);
CREATE INDEX "index_channel_threads_on_creator_id" ON "channel_threads" ("creator_id");
CREATE UNIQUE INDEX "index_channel_threads_on_parent_message_id" ON "channel_threads" ("parent_message_id") WHERE parent_message_id IS NOT NULL;
CREATE INDEX "index_channel_threads_on_room_id_and_closed_at_and_locked_at" ON "channel_threads" ("room_id", "closed_at", "locked_at");
CREATE INDEX "index_channel_threads_on_room_id_and_last_activity_at" ON "channel_threads" ("room_id", "last_activity_at");
CREATE INDEX "index_channel_threads_on_room_and_work_status_and_activity" ON "channel_threads" ("room_id", "work_status", "last_activity_at");
CREATE INDEX "index_channel_threads_on_work_owner_id" ON "channel_threads" ("work_owner_id");
CREATE TABLE "dnd_allowed_users" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "allowed_user_id" integer NOT NULL, "created_at" datetime(6) NOT NULL, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_aa536dbe6b"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
, CONSTRAINT "fk_rails_2ba68c0017"
FOREIGN KEY ("allowed_user_id")
  REFERENCES "users" ("id")
);
CREATE INDEX "index_dnd_allowed_users_on_allowed_user_id" ON "dnd_allowed_users" ("allowed_user_id");
CREATE UNIQUE INDEX "index_dnd_allowed_users_on_user_id_and_allowed_user_id" ON "dnd_allowed_users" ("user_id", "allowed_user_id");
CREATE INDEX "index_dnd_allowed_users_on_user_id" ON "dnd_allowed_users" ("user_id");
CREATE TABLE "drive_attachments" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "file_id" varchar NOT NULL, "message_id" integer NOT NULL, CONSTRAINT "fk_rails_52985ea38c"
FOREIGN KEY ("message_id")
  REFERENCES "messages" ("id")
);
CREATE UNIQUE INDEX "index_drive_attachments_on_message_id_and_file_id" ON "drive_attachments" ("message_id", "file_id");
CREATE INDEX "index_drive_attachments_on_message_id" ON "drive_attachments" ("message_id");
CREATE TABLE "event_calendar_entries" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "event_id" integer NOT NULL, "google_event_id" varchar NOT NULL, "last_error" varchar, "synced_at" datetime(6), "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_746e01925a"
FOREIGN KEY ("event_id")
  REFERENCES "events" ("id")
, CONSTRAINT "fk_rails_0204504c82"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE UNIQUE INDEX "index_event_calendar_entries_on_event_id_and_user_id" ON "event_calendar_entries" ("event_id", "user_id");
CREATE INDEX "index_event_calendar_entries_on_event_id" ON "event_calendar_entries" ("event_id");
CREATE INDEX "index_event_calendar_entries_on_user_id" ON "event_calendar_entries" ("user_id");
CREATE TABLE "event_references" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "event_id" integer NOT NULL, "message_id" integer NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_f1ec8b596e"
FOREIGN KEY ("event_id")
  REFERENCES "events" ("id")
, CONSTRAINT "fk_rails_134bd7e1f9"
FOREIGN KEY ("message_id")
  REFERENCES "messages" ("id")
);
CREATE INDEX "index_event_references_on_event_id" ON "event_references" ("event_id");
CREATE UNIQUE INDEX "index_event_references_on_message_id_and_event_id" ON "event_references" ("message_id", "event_id");
CREATE INDEX "index_event_references_on_message_id" ON "event_references" ("message_id");
CREATE TABLE "fizzy_card_caches" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "fetch_error" varchar, "fetch_requested_at" datetime(6), "fetched_at" datetime(6), "fizzy_card_id" integer NOT NULL, "payload" json, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_2d6a9898d4"
FOREIGN KEY ("fizzy_card_id")
  REFERENCES "fizzy_cards" ("id")
, CONSTRAINT "fk_rails_91fb0f0b3b"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE UNIQUE INDEX "index_fizzy_card_caches_on_card_and_user" ON "fizzy_card_caches" ("fizzy_card_id", "user_id");
CREATE INDEX "index_fizzy_card_caches_on_fizzy_card_id" ON "fizzy_card_caches" ("fizzy_card_id");
CREATE INDEX "index_fizzy_card_caches_on_user_id" ON "fizzy_card_caches" ("user_id");
CREATE TABLE "fizzy_card_references" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "fizzy_card_id" integer NOT NULL, "message_id" integer NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_d27c7dea58"
FOREIGN KEY ("fizzy_card_id")
  REFERENCES "fizzy_cards" ("id")
, CONSTRAINT "fk_rails_d37d8cd7eb"
FOREIGN KEY ("message_id")
  REFERENCES "messages" ("id")
);
CREATE INDEX "index_fizzy_card_references_on_fizzy_card_id" ON "fizzy_card_references" ("fizzy_card_id");
CREATE UNIQUE INDEX "index_fizzy_card_refs_on_message_and_card" ON "fizzy_card_references" ("message_id", "fizzy_card_id");
CREATE INDEX "index_fizzy_card_references_on_message_id" ON "fizzy_card_references" ("message_id");
CREATE TABLE "fizzy_connected_accounts" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "access_token" varchar NOT NULL, "created_at" datetime(6) NOT NULL, "disconnected_reason" varchar, "fizzy_account_id" varchar NOT NULL, "fizzy_account_name" varchar, "fizzy_user_id" varchar, "fizzy_user_name" varchar, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_3bdcf24ac0"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE UNIQUE INDEX "index_fizzy_connected_accounts_on_user_id" ON "fizzy_connected_accounts" ("user_id");
CREATE TABLE "github_connected_accounts" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "access_token" varchar NOT NULL, "created_at" datetime(6) NOT NULL, "disconnected_reason" varchar, "github_login" varchar NOT NULL, "last_error" varchar, "refresh_token" varchar, "token_expires_at" datetime(6), "token_source" varchar DEFAULT 'pat' NOT NULL, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_018307ebeb"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE UNIQUE INDEX "index_github_connected_accounts_on_user_id" ON "github_connected_accounts" ("user_id");
CREATE TABLE "github_notifications" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "dedupe_key" varchar NOT NULL, "message_id" integer, "subscription_id" integer NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_fcd6637a2f"
FOREIGN KEY ("subscription_id")
  REFERENCES "github_repository_subscriptions" ("id")
 ON DELETE CASCADE, CONSTRAINT "fk_rails_f6aee4f9b2"
FOREIGN KEY ("message_id")
  REFERENCES "messages" ("id")
 ON DELETE SET NULL);
CREATE INDEX "index_github_notifications_on_message_id" ON "github_notifications" ("message_id");
CREATE UNIQUE INDEX "index_github_notifications_on_subscription_and_key" ON "github_notifications" ("subscription_id", "dedupe_key");
CREATE INDEX "index_github_notifications_on_subscription_id" ON "github_notifications" ("subscription_id");
CREATE TABLE "github_pull_request_references" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "github_pull_request_id" integer NOT NULL, "message_id" integer NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_66258ff416"
FOREIGN KEY ("github_pull_request_id")
  REFERENCES "github_pull_requests" ("id")
, CONSTRAINT "fk_rails_3d019d8312"
FOREIGN KEY ("message_id")
  REFERENCES "messages" ("id")
);
CREATE INDEX "index_github_pull_request_references_on_github_pull_request_id" ON "github_pull_request_references" ("github_pull_request_id");
CREATE UNIQUE INDEX "index_gh_pr_refs_on_message_and_pr" ON "github_pull_request_references" ("message_id", "github_pull_request_id");
CREATE INDEX "index_github_pull_request_references_on_message_id" ON "github_pull_request_references" ("message_id");
CREATE TABLE "github_pull_request_threads" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "channel_thread_id" integer NOT NULL, "created_at" datetime(6) NOT NULL, "github_pull_request_id" integer NOT NULL, "room_id" integer NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_17af2a3f6d"
FOREIGN KEY ("github_pull_request_id")
  REFERENCES "github_pull_requests" ("id")
, CONSTRAINT "fk_rails_2e882c47f0"
FOREIGN KEY ("channel_thread_id")
  REFERENCES "channel_threads" ("id")
, CONSTRAINT "fk_rails_891a4ab7f9"
FOREIGN KEY ("room_id")
  REFERENCES "rooms" ("id")
);
CREATE UNIQUE INDEX "index_github_pr_threads_on_thread" ON "github_pull_request_threads" ("channel_thread_id");
CREATE INDEX "index_github_pull_request_threads_on_channel_thread_id" ON "github_pull_request_threads" ("channel_thread_id");
CREATE UNIQUE INDEX "index_github_pr_threads_on_pr_and_room" ON "github_pull_request_threads" ("github_pull_request_id", "room_id");
CREATE INDEX "index_github_pull_request_threads_on_github_pull_request_id" ON "github_pull_request_threads" ("github_pull_request_id");
CREATE INDEX "index_github_pull_request_threads_on_room_id" ON "github_pull_request_threads" ("room_id");
CREATE TABLE "github_repository_subscriptions" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "created_by_id" integer, "events" json DEFAULT '[]' NOT NULL, "owner" varchar NOT NULL, "reader_verified" boolean DEFAULT FALSE NOT NULL, "repo" varchar NOT NULL, "room_id" integer NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_07236e948a"
FOREIGN KEY ("room_id")
  REFERENCES "rooms" ("id")
 ON DELETE CASCADE, CONSTRAINT "fk_rails_f6cd06a625"
FOREIGN KEY ("created_by_id")
  REFERENCES "users" ("id")
 ON DELETE SET NULL);
CREATE INDEX "index_github_repository_subscriptions_on_created_by_id" ON "github_repository_subscriptions" ("created_by_id");
CREATE INDEX "index_github_subscriptions_on_owner_and_repo" ON "github_repository_subscriptions" ("owner", "repo");
CREATE UNIQUE INDEX "index_github_subscriptions_on_room_and_repo" ON "github_repository_subscriptions" ("room_id", "owner", "repo");
CREATE INDEX "index_github_repository_subscriptions_on_room_id" ON "github_repository_subscriptions" ("room_id");
CREATE TABLE "google_accounts" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "access_token" varchar, "access_token_expires_at" datetime(6), "created_at" datetime(6) NOT NULL, "disconnected_reason" varchar, "email" varchar NOT NULL, "refresh_token" varchar, "scopes" varchar, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_2587a26a49"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE UNIQUE INDEX "index_google_accounts_on_user_id" ON "google_accounts" ("user_id");
CREATE TABLE "google_identities" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "domain" varchar, "email" varchar NOT NULL, "subject" varchar NOT NULL, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_21846cb8d2"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE UNIQUE INDEX "index_google_identities_on_subject" ON "google_identities" ("subject");
CREATE UNIQUE INDEX "index_google_identities_on_user_id" ON "google_identities" ("user_id");
CREATE TABLE "keyword_alerts" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "phrase" varchar NOT NULL, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_0de64fe97d"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE UNIQUE INDEX "index_keyword_alerts_on_user_id_and_phrase" ON "keyword_alerts" ("user_id", "phrase");
CREATE INDEX "index_keyword_alerts_on_user_id" ON "keyword_alerts" ("user_id");
CREATE TABLE "link_embed_references" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "link_embed_id" integer NOT NULL, "message_id" integer NOT NULL, "position" integer DEFAULT 0 NOT NULL, "updated_at" datetime(6) NOT NULL, "url" varchar, CONSTRAINT "fk_rails_656df5f01b"
FOREIGN KEY ("link_embed_id")
  REFERENCES "link_embeds" ("id")
, CONSTRAINT "fk_rails_eaf3a8aa25"
FOREIGN KEY ("message_id")
  REFERENCES "messages" ("id")
);
CREATE INDEX "index_link_embed_references_on_link_embed_id" ON "link_embed_references" ("link_embed_id");
CREATE UNIQUE INDEX "index_link_embed_references_on_message_and_embed" ON "link_embed_references" ("message_id", "link_embed_id");
CREATE INDEX "index_link_embed_references_on_message_id" ON "link_embed_references" ("message_id");
CREATE TABLE "message_pins" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "message_id" integer NOT NULL, "pinner_id" integer NOT NULL, "room_id" integer NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_674c7939be"
FOREIGN KEY ("room_id")
  REFERENCES "rooms" ("id")
, CONSTRAINT "fk_rails_8609095c5f"
FOREIGN KEY ("message_id")
  REFERENCES "messages" ("id")
, CONSTRAINT "fk_rails_9d78ec0659"
FOREIGN KEY ("pinner_id")
  REFERENCES "users" ("id")
);
CREATE UNIQUE INDEX "index_message_pins_on_message_id" ON "message_pins" ("message_id");
CREATE INDEX "index_message_pins_on_pinner_id" ON "message_pins" ("pinner_id");
CREATE INDEX "index_message_pins_on_room_id_and_created_at" ON "message_pins" ("room_id", "created_at");
CREATE INDEX "index_message_pins_on_room_id" ON "message_pins" ("room_id");
CREATE TABLE "message_references" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "message_id" integer NOT NULL, "referenced_message_id" integer NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_417f410a76"
FOREIGN KEY ("message_id")
  REFERENCES "messages" ("id")
, CONSTRAINT "fk_rails_c6a8ce6563"
FOREIGN KEY ("referenced_message_id")
  REFERENCES "messages" ("id")
);
CREATE UNIQUE INDEX "index_message_references_on_message_and_referenced" ON "message_references" ("message_id", "referenced_message_id");
CREATE INDEX "index_message_references_on_message_id" ON "message_references" ("message_id");
CREATE INDEX "index_message_references_on_referenced_message_id" ON "message_references" ("referenced_message_id");
CREATE TABLE "messages" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "action" boolean DEFAULT FALSE NOT NULL, "board_post_opener" boolean DEFAULT FALSE NOT NULL, "client_message_id" varchar NOT NULL, "created_at" datetime(6) NOT NULL, "creator_id" integer NOT NULL, "edited_at" datetime(6), "embeds_suppressed" boolean DEFAULT FALSE NOT NULL, "forward_note" text, "forwarded_at" datetime(6), "forwarded_from_message_id" integer, "forwarded_markdown" boolean DEFAULT FALSE NOT NULL, "markdown_source" text, "reply_notify_author" boolean DEFAULT TRUE NOT NULL, "reply_target_deleted_at" datetime(6), "reply_to_message_id" integer, "room_id" integer NOT NULL, "stream_broadcast_at" datetime(6), "streaming" boolean DEFAULT FALSE NOT NULL, "streaming_updated_at" datetime(6), "system_note" boolean DEFAULT FALSE NOT NULL, "thread_id" integer, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_a8db0fb63a"
FOREIGN KEY ("room_id")
  REFERENCES "rooms" ("id")
, CONSTRAINT "fk_rails_6d240335a7"
FOREIGN KEY ("forwarded_from_message_id")
  REFERENCES "messages" ("id")
 ON DELETE SET NULL, CONSTRAINT "fk_rails_3ff3321a9c"
FOREIGN KEY ("thread_id")
  REFERENCES "channel_threads" ("id")
 ON DELETE CASCADE, CONSTRAINT "fk_rails_0348d0c85c"
FOREIGN KEY ("reply_to_message_id")
  REFERENCES "messages" ("id")
 ON DELETE SET NULL, CONSTRAINT "fk_rails_761a2f12b3"
FOREIGN KEY ("creator_id")
  REFERENCES "users" ("id")
);
CREATE INDEX "index_messages_on_creator_id" ON "messages" ("creator_id");
CREATE INDEX "index_messages_on_forwarded_from_message_id" ON "messages" ("forwarded_from_message_id");
CREATE INDEX "index_messages_on_reply_to_message_id" ON "messages" ("reply_to_message_id");
CREATE INDEX "index_messages_on_room_creator_client_id" ON "messages" ("room_id", "creator_id", "client_message_id");
CREATE INDEX "index_messages_on_room_thread_created" ON "messages" ("room_id", "thread_id", "created_at");
CREATE INDEX "index_messages_on_room_id" ON "messages" ("room_id");
CREATE INDEX "index_messages_on_streaming_and_created_at" ON "messages" ("streaming", "created_at") WHERE streaming;
CREATE INDEX "index_messages_on_streaming_updated_at" ON "messages" ("streaming_updated_at") WHERE streaming = 1;
CREATE INDEX "index_messages_on_thread_created" ON "messages" ("thread_id", "created_at");
CREATE INDEX "index_messages_on_thread_id" ON "messages" ("thread_id");
CREATE TABLE "poll_options" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "label" varchar NOT NULL, "poll_id" integer NOT NULL, "position" integer DEFAULT 0 NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_aa85becb42"
FOREIGN KEY ("poll_id")
  REFERENCES "polls" ("id")
);
CREATE INDEX "index_poll_options_on_poll_id_and_position" ON "poll_options" ("poll_id", "position");
CREATE INDEX "index_poll_options_on_poll_id" ON "poll_options" ("poll_id");
CREATE TABLE "poll_votes" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "poll_id" integer NOT NULL, "poll_option_id" integer NOT NULL, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_a6e6974b7e"
FOREIGN KEY ("poll_id")
  REFERENCES "polls" ("id")
, CONSTRAINT "fk_rails_848ece0184"
FOREIGN KEY ("poll_option_id")
  REFERENCES "poll_options" ("id")
, CONSTRAINT "fk_rails_b64de9b025"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE INDEX "index_poll_votes_on_poll_id_and_user_id" ON "poll_votes" ("poll_id", "user_id");
CREATE INDEX "index_poll_votes_on_poll_id" ON "poll_votes" ("poll_id");
CREATE UNIQUE INDEX "index_poll_votes_on_poll_option_id_and_user_id" ON "poll_votes" ("poll_option_id", "user_id");
CREATE INDEX "index_poll_votes_on_poll_option_id" ON "poll_votes" ("poll_option_id");
CREATE INDEX "index_poll_votes_on_user_id" ON "poll_votes" ("user_id");
CREATE TABLE "polls" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "anonymous" boolean DEFAULT FALSE NOT NULL, "closed_at" datetime(6), "closes_at" datetime(6), "created_at" datetime(6) NOT NULL, "message_id" integer NOT NULL, "multiple" boolean DEFAULT FALSE NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_09aa6c0c53"
FOREIGN KEY ("message_id")
  REFERENCES "messages" ("id")
);
CREATE UNIQUE INDEX "index_polls_on_message_id" ON "polls" ("message_id");
CREATE TABLE "push_subscriptions" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "auth_key" varchar, "created_at" datetime(6) NOT NULL, "endpoint" varchar, "p256dh_key" varchar, "updated_at" datetime(6) NOT NULL, "user_agent" varchar, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_43d43720fc"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE INDEX "idx_on_endpoint_p256dh_key_auth_key_7553014576" ON "push_subscriptions" ("endpoint", "p256dh_key", "auth_key");
CREATE INDEX "index_push_subscriptions_on_user_id" ON "push_subscriptions" ("user_id");
CREATE TABLE "saved_items" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "message_id" integer NOT NULL, "remind_at" datetime(6), "reminded_at" datetime(6), "status" varchar DEFAULT 'in_progress' NOT NULL, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_b06751c480"
FOREIGN KEY ("message_id")
  REFERENCES "messages" ("id")
, CONSTRAINT "fk_rails_1db686816f"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE INDEX "index_saved_items_on_message_id" ON "saved_items" ("message_id");
CREATE INDEX "index_saved_items_on_remind_at" ON "saved_items" ("remind_at") WHERE remind_at IS NOT NULL AND reminded_at IS NULL;
CREATE UNIQUE INDEX "index_saved_items_on_user_id_and_message_id" ON "saved_items" ("user_id", "message_id");
CREATE INDEX "index_saved_items_on_user_id_and_status" ON "saved_items" ("user_id", "status");
CREATE INDEX "index_saved_items_on_user_id" ON "saved_items" ("user_id");
CREATE TABLE "scheduled_messages" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "claimed_at" datetime(6), "created_at" datetime(6) NOT NULL, "drop_reason" text, "dropped_at" datetime(6), "markdown_source" text NOT NULL, "reply_to_message_id" integer, "room_id" integer NOT NULL, "send_at" datetime(6) NOT NULL, "sent_at" datetime(6), "sent_message_id" integer, "thread_id" integer, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_ddb11275ba"
FOREIGN KEY ("room_id")
  REFERENCES "rooms" ("id")
, CONSTRAINT "fk_rails_3465c6cd26"
FOREIGN KEY ("reply_to_message_id")
  REFERENCES "messages" ("id")
 ON DELETE SET NULL, CONSTRAINT "fk_rails_e1d5fc400c"
FOREIGN KEY ("thread_id")
  REFERENCES "channel_threads" ("id")
 ON DELETE SET NULL, CONSTRAINT "fk_rails_c6b9a0ec8d"
FOREIGN KEY ("sent_message_id")
  REFERENCES "messages" ("id")
 ON DELETE SET NULL, CONSTRAINT "fk_rails_6856b1c1c3"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE INDEX "index_scheduled_messages_on_reply_to_message_id" ON "scheduled_messages" ("reply_to_message_id");
CREATE INDEX "index_scheduled_messages_on_room_id" ON "scheduled_messages" ("room_id");
CREATE INDEX "index_scheduled_messages_on_send_at_and_sent_at_and_dropped_at" ON "scheduled_messages" ("send_at", "sent_at", "dropped_at");
CREATE INDEX "index_scheduled_messages_on_sent_message_id" ON "scheduled_messages" ("sent_message_id");
CREATE INDEX "index_scheduled_messages_on_thread_id" ON "scheduled_messages" ("thread_id");
CREATE INDEX "index_scheduled_messages_on_user_id_and_send_at" ON "scheduled_messages" ("user_id", "send_at");
CREATE INDEX "index_scheduled_messages_on_user_id" ON "scheduled_messages" ("user_id");
CREATE TABLE "searches" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "dedup_key" varchar, "query" varchar NOT NULL, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_e192b86393"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE UNIQUE INDEX "index_searches_on_user_and_dedup_key" ON "searches" ("user_id", "dedup_key");
CREATE INDEX "index_searches_on_user_id" ON "searches" ("user_id");
CREATE TABLE "sessions" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "device_id" varchar, "ip_address" varchar, "last_active_at" datetime(6) NOT NULL, "token" varchar NOT NULL, "two_factor_verified_at" datetime(6), "updated_at" datetime(6) NOT NULL, "user_agent" varchar, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_758836b4f0"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE UNIQUE INDEX "index_sessions_on_token" ON "sessions" ("token");
CREATE INDEX "index_sessions_on_user_id_and_device_id" ON "sessions" ("user_id", "device_id");
CREATE INDEX "index_sessions_on_user_id" ON "sessions" ("user_id");
CREATE TABLE "slack_connections" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "access_token" text, "created_at" datetime(6) NOT NULL, "disconnected_reason" varchar, "scopes" varchar, "slack_user_id" varchar NOT NULL, "slack_workspace_id" integer NOT NULL, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_a648790e67"
FOREIGN KEY ("slack_workspace_id")
  REFERENCES "slack_workspaces" ("id")
, CONSTRAINT "fk_rails_4a9e4ba1b7"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE UNIQUE INDEX "idx_on_slack_workspace_id_slack_user_id_cde7a4e4d3" ON "slack_connections" ("slack_workspace_id", "slack_user_id");
CREATE INDEX "index_slack_connections_on_slack_workspace_id" ON "slack_connections" ("slack_workspace_id");
CREATE UNIQUE INDEX "index_slack_connections_on_user_id" ON "slack_connections" ("user_id");
CREATE TABLE "slack_import_issues" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "level" varchar NOT NULL, "message" text NOT NULL, "slack_import_id" integer NOT NULL, "slack_ref" varchar, CONSTRAINT "fk_rails_a1e667e12e"
FOREIGN KEY ("slack_import_id")
  REFERENCES "slack_imports" ("id")
);
CREATE INDEX "index_slack_import_issues_on_slack_import_id" ON "slack_import_issues" ("slack_import_id");
CREATE TABLE "slack_import_records" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "created_record" boolean DEFAULT TRUE NOT NULL, "record_id" bigint NOT NULL, "record_type" varchar NOT NULL, "slack_import_id" integer NOT NULL, "slack_key" varchar NOT NULL, "slack_kind" varchar NOT NULL, "slack_workspace_id" integer NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_6a6804083f"
FOREIGN KEY ("slack_import_id")
  REFERENCES "slack_imports" ("id")
, CONSTRAINT "fk_rails_d25fcffaef"
FOREIGN KEY ("slack_workspace_id")
  REFERENCES "slack_workspaces" ("id")
);
CREATE INDEX "index_slack_import_records_on_record_type_and_record_id" ON "slack_import_records" ("record_type", "record_id");
CREATE INDEX "index_slack_import_records_on_slack_import_id" ON "slack_import_records" ("slack_import_id");
CREATE UNIQUE INDEX "index_slack_import_records_on_slack_identity" ON "slack_import_records" ("slack_workspace_id", "slack_kind", "slack_key");
CREATE INDEX "index_slack_import_records_on_slack_workspace_id" ON "slack_import_records" ("slack_workspace_id");
CREATE TABLE "slack_imports" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "error" text, "finished_at" datetime(6), "heartbeat_at" datetime(6), "kind" varchar NOT NULL, "mode" varchar NOT NULL, "options" json DEFAULT '{}' NOT NULL, "slack_connection_id" integer, "slack_workspace_id" integer NOT NULL, "started_at" datetime(6), "state" json DEFAULT '{}' NOT NULL, "stats" json DEFAULT '{}' NOT NULL, "status" varchar DEFAULT 'queued' NOT NULL, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_d8efa28220"
FOREIGN KEY ("slack_workspace_id")
  REFERENCES "slack_workspaces" ("id")
, CONSTRAINT "fk_rails_bb3b3a0ded"
FOREIGN KEY ("slack_connection_id")
  REFERENCES "slack_connections" ("id")
 ON DELETE SET NULL, CONSTRAINT "fk_rails_c191ba8368"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE INDEX "index_slack_imports_on_slack_connection_id" ON "slack_imports" ("slack_connection_id");
CREATE INDEX "index_slack_imports_on_slack_workspace_id" ON "slack_imports" ("slack_workspace_id");
CREATE INDEX "index_slack_imports_on_status" ON "slack_imports" ("status");
CREATE INDEX "index_slack_imports_on_user_id" ON "slack_imports" ("user_id");
CREATE TABLE "slack_workspaces" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "client_id" varchar, "client_secret" text, "configured_by_id" integer, "created_at" datetime(6) NOT NULL, "team_domain" varchar, "team_id" varchar, "team_name" varchar, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_bb0ac666f4"
FOREIGN KEY ("configured_by_id")
  REFERENCES "users" ("id")
);
CREATE INDEX "index_slack_workspaces_on_configured_by_id" ON "slack_workspaces" ("configured_by_id");
CREATE UNIQUE INDEX "index_slack_workspaces_on_team_id" ON "slack_workspaces" ("team_id");
CREATE TABLE "thread_memberships" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "involvement" varchar DEFAULT 'mentions' NOT NULL, "joined_at" datetime(6) NOT NULL, "thread_id" integer NOT NULL, "unread_at" datetime(6), "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, "last_read_message_id" integer, CONSTRAINT "fk_rails_21cece547a"
FOREIGN KEY ("thread_id")
  REFERENCES "channel_threads" ("id")
 ON DELETE CASCADE, CONSTRAINT "fk_rails_00c95cb16e"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
 ON DELETE CASCADE);
CREATE INDEX "index_thread_memberships_on_thread_id_and_unread_at" ON "thread_memberships" ("thread_id", "unread_at");
CREATE UNIQUE INDEX "index_thread_memberships_on_thread_id_and_user_id" ON "thread_memberships" ("thread_id", "user_id");
CREATE INDEX "index_thread_memberships_on_user_id" ON "thread_memberships" ("user_id");
CREATE TABLE "twitter_post_references" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "message_id" integer NOT NULL, "twitter_post_id" integer NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_2ac06442b9"
FOREIGN KEY ("message_id")
  REFERENCES "messages" ("id")
, CONSTRAINT "fk_rails_afa0c7dc89"
FOREIGN KEY ("twitter_post_id")
  REFERENCES "twitter_posts" ("id")
);
CREATE UNIQUE INDEX "index_twitter_post_references_on_message_and_post" ON "twitter_post_references" ("message_id", "twitter_post_id");
CREATE INDEX "index_twitter_post_references_on_message_id" ON "twitter_post_references" ("message_id");
CREATE INDEX "index_twitter_post_references_on_twitter_post_id" ON "twitter_post_references" ("twitter_post_id");
CREATE TABLE "two_factor_backup_codes" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "code_digest" varchar NOT NULL, "created_at" datetime(6) NOT NULL, "two_factor_credential_id" integer NOT NULL, "updated_at" datetime(6) NOT NULL, "used_at" datetime(6), CONSTRAINT "fk_rails_fdbdba3cfc"
FOREIGN KEY ("two_factor_credential_id")
  REFERENCES "two_factor_credentials" ("id")
);
CREATE UNIQUE INDEX "index_two_factor_backup_codes_on_code_digest" ON "two_factor_backup_codes" ("code_digest");
CREATE INDEX "index_two_factor_backup_codes_on_two_factor_credential_id" ON "two_factor_backup_codes" ("two_factor_credential_id");
CREATE TABLE "two_factor_credentials" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "confirmed_at" datetime(6), "consecutive_failures" integer DEFAULT 0 NOT NULL, "created_at" datetime(6) NOT NULL, "last_totp_at" bigint, "locked_until" datetime(6), "lockout_count" integer DEFAULT 0 NOT NULL, "secret" varchar, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_5a3294fb16"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE UNIQUE INDEX "index_two_factor_credentials_on_user_id" ON "two_factor_credentials" ("user_id");
CREATE TABLE "two_factor_remembered_devices" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "expires_at" datetime(6) NOT NULL, "ip_address" varchar, "last_used_at" datetime(6), "token_digest" varchar NOT NULL, "updated_at" datetime(6) NOT NULL, "user_agent" varchar(512), "user_id" integer NOT NULL, CONSTRAINT "fk_rails_15d9964e51"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE UNIQUE INDEX "index_two_factor_remembered_devices_on_token_digest" ON "two_factor_remembered_devices" ("token_digest");
CREATE INDEX "index_two_factor_remembered_devices_on_user_id" ON "two_factor_remembered_devices" ("user_id");
CREATE TABLE "two_factor_setup_secrets" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "expires_at" datetime(6) NOT NULL, "secret" varchar, "session_id" integer NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_ce24fc2c93"
FOREIGN KEY ("session_id")
  REFERENCES "sessions" ("id")
 ON DELETE CASCADE);
CREATE UNIQUE INDEX "index_two_factor_setup_secrets_on_session_id" ON "two_factor_setup_secrets" ("session_id");
CREATE TABLE "user_devices" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "device_id" varchar NOT NULL, "updated_at" datetime(6) NOT NULL, "user_agent" varchar, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_e700a96826"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
 ON DELETE CASCADE);
CREATE UNIQUE INDEX "index_user_devices_on_user_id_and_device_id" ON "user_devices" ("user_id", "device_id");
CREATE INDEX "index_user_devices_on_user_id" ON "user_devices" ("user_id");
CREATE TABLE "user_stars" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "starred_user_id" integer NOT NULL, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_573ad1a98c"
FOREIGN KEY ("starred_user_id")
  REFERENCES "users" ("id")
 ON DELETE CASCADE, CONSTRAINT "fk_rails_756c2c1f92"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
 ON DELETE CASCADE);
CREATE INDEX "index_user_stars_on_starred_user_id" ON "user_stars" ("starred_user_id");
CREATE UNIQUE INDEX "index_user_stars_on_user_id_and_starred_user_id" ON "user_stars" ("user_id", "starred_user_id");
CREATE INDEX "index_user_stars_on_user_id" ON "user_stars" ("user_id");
CREATE TABLE "webhooks" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "signing_secret" varchar, "updated_at" datetime(6) NOT NULL, "url" varchar, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_51bf96d3bc"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
);
CREATE INDEX "index_webhooks_on_user_id" ON "webhooks" ("user_id");
CREATE TABLE "work_handoffs" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "channel_thread_id" integer NOT NULL, "created_at" datetime(6) NOT NULL, "links" json DEFAULT '[]' NOT NULL, "open_questions" json DEFAULT '[]' NOT NULL, "receiver_agent_id" integer NOT NULL, "sender_id" integer NOT NULL, "summary" text NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_d07b7d12b4"
FOREIGN KEY ("channel_thread_id")
  REFERENCES "channel_threads" ("id")
, CONSTRAINT "fk_rails_d6c5673495"
FOREIGN KEY ("receiver_agent_id")
  REFERENCES "agents" ("id")
, CONSTRAINT "fk_rails_c128b56a79"
FOREIGN KEY ("sender_id")
  REFERENCES "users" ("id")
);
CREATE INDEX "index_work_handoffs_on_channel_thread_id_and_created_at" ON "work_handoffs" ("channel_thread_id", "created_at");
CREATE INDEX "index_work_handoffs_on_channel_thread_id" ON "work_handoffs" ("channel_thread_id");
CREATE INDEX "index_work_handoffs_on_receiver_agent_id" ON "work_handoffs" ("receiver_agent_id");
CREATE INDEX "index_work_handoffs_on_sender_id" ON "work_handoffs" ("sender_id");
CREATE TABLE "work_thread_events" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "actor_id" integer, "channel_thread_id" integer NOT NULL, "created_at" datetime(6) NOT NULL, "event_type" varchar NOT NULL, "from_owner_id" integer, "from_owner_name" varchar, "from_status" varchar, "metadata" json, "to_owner_id" integer, "to_owner_name" varchar, "to_status" varchar, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_b57380a040"
FOREIGN KEY ("channel_thread_id")
  REFERENCES "channel_threads" ("id")
 ON DELETE CASCADE, CONSTRAINT "fk_rails_8682a22e18"
FOREIGN KEY ("actor_id")
  REFERENCES "users" ("id")
 ON DELETE SET NULL);
CREATE INDEX "index_work_thread_events_on_actor_id" ON "work_thread_events" ("actor_id");
CREATE INDEX "index_work_thread_events_on_thread_and_created_at" ON "work_thread_events" ("channel_thread_id", "created_at");
CREATE INDEX "index_work_thread_events_on_channel_thread_id" ON "work_thread_events" ("channel_thread_id");
CREATE INDEX "index_work_thread_events_on_type_and_created_at" ON "work_thread_events" ("event_type", "created_at");
CREATE TABLE "work_thread_links" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "channel_thread_id" integer NOT NULL, "created_at" datetime(6) NOT NULL, "created_by_id" integer NOT NULL, "event_id" integer, "github_pull_request_id" integer, "kind" varchar NOT NULL, "title" varchar, "updated_at" datetime(6) NOT NULL, "url" varchar, CONSTRAINT "fk_rails_b976dd70c0"
FOREIGN KEY ("github_pull_request_id")
  REFERENCES "github_pull_requests" ("id")
, CONSTRAINT "fk_rails_e1e2ace780"
FOREIGN KEY ("channel_thread_id")
  REFERENCES "channel_threads" ("id")
 ON DELETE CASCADE, CONSTRAINT "fk_rails_258fd15cfa"
FOREIGN KEY ("event_id")
  REFERENCES "events" ("id")
 ON DELETE CASCADE, CONSTRAINT "fk_rails_ae8dcd3938"
FOREIGN KEY ("created_by_id")
  REFERENCES "users" ("id")
);
CREATE UNIQUE INDEX "index_work_thread_links_on_thread_and_event" ON "work_thread_links" ("channel_thread_id", "event_id") WHERE event_id IS NOT NULL;
CREATE UNIQUE INDEX "index_work_thread_links_on_thread_kind_and_pr" ON "work_thread_links" ("channel_thread_id", "kind", "github_pull_request_id") WHERE github_pull_request_id IS NOT NULL;
CREATE UNIQUE INDEX "index_work_thread_links_on_thread_and_url" ON "work_thread_links" ("channel_thread_id", "url") WHERE url IS NOT NULL;
CREATE INDEX "index_work_thread_links_on_channel_thread_id" ON "work_thread_links" ("channel_thread_id");
CREATE INDEX "index_work_thread_links_on_created_by_id" ON "work_thread_links" ("created_by_id");
CREATE INDEX "index_work_thread_links_on_event_id" ON "work_thread_links" ("event_id");
CREATE INDEX "index_work_thread_links_on_github_pull_request_id" ON "work_thread_links" ("github_pull_request_id");
CREATE TABLE "workspace_icons" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "created_at" datetime(6) NOT NULL, "creator_id" integer NOT NULL, "name" varchar NOT NULL, "title" varchar NOT NULL, "updated_at" datetime(6) NOT NULL, CONSTRAINT "fk_rails_991036da9f"
FOREIGN KEY ("creator_id")
  REFERENCES "users" ("id")
);
CREATE INDEX "index_workspace_icons_on_creator_id" ON "workspace_icons" ("creator_id");
CREATE UNIQUE INDEX "index_workspace_icons_on_name" ON "workspace_icons" ("name");
CREATE TABLE "workspace_presence_leases" ("id" integer PRIMARY KEY AUTOINCREMENT NOT NULL, "connection_id" varchar NOT NULL, "created_at" datetime(6) NOT NULL, "expires_at" datetime(6) NOT NULL, "last_active_at" datetime(6), "session_id" integer NOT NULL, "updated_at" datetime(6) NOT NULL, "user_id" integer NOT NULL, CONSTRAINT "fk_rails_029856a6c3"
FOREIGN KEY ("session_id")
  REFERENCES "sessions" ("id")
 ON DELETE CASCADE, CONSTRAINT "fk_rails_b16164ece4"
FOREIGN KEY ("user_id")
  REFERENCES "users" ("id")
 ON DELETE CASCADE);
CREATE UNIQUE INDEX "index_workspace_presence_leases_on_connection_id" ON "workspace_presence_leases" ("connection_id");
CREATE INDEX "index_workspace_presence_leases_on_expires_at" ON "workspace_presence_leases" ("expires_at");
CREATE INDEX "index_workspace_presence_leases_on_session_id" ON "workspace_presence_leases" ("session_id");
CREATE INDEX "index_workspace_presence_leases_on_user_id" ON "workspace_presence_leases" ("user_id");
CREATE VIRTUAL TABLE message_search_index USING fts5 (body, tokenize=porter);
CREATE TABLE "schema_migrations" ("version" varchar NOT NULL PRIMARY KEY);
CREATE TABLE "ar_internal_metadata" ("key" varchar NOT NULL PRIMARY KEY, "value" varchar, "created_at" datetime(6) NOT NULL, "updated_at" datetime(6) NOT NULL);
CREATE TRIGGER activity_revision_activity_items_insert
AFTER INSERT ON activity_items
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id = NEW.user_id;
END;
CREATE TRIGGER activity_revision_activity_items_delete
BEFORE DELETE ON activity_items
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id = OLD.user_id;
END;
CREATE TRIGGER activity_revision_activity_items_update
AFTER UPDATE OF user_id, source_type, source_id, event_type, read_at, handled_at, updated_at ON activity_items
WHEN OLD.user_id IS NOT NEW.user_id OR OLD.source_type IS NOT NEW.source_type OR OLD.source_id IS NOT NEW.source_id OR OLD.event_type IS NOT NEW.event_type OR OLD.read_at IS NOT NEW.read_at OR OLD.handled_at IS NOT NEW.handled_at OR OLD.updated_at IS NOT NEW.updated_at
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id = OLD.user_id) OR (id = NEW.user_id);
END;
CREATE TRIGGER activity_revision_memberships_insert
AFTER INSERT ON memberships
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id = NEW.user_id;
END;
CREATE TRIGGER activity_revision_memberships_delete
BEFORE DELETE ON memberships
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id = OLD.user_id;
END;
CREATE TRIGGER activity_revision_memberships_update
AFTER UPDATE OF id, user_id, room_id ON memberships
WHEN OLD.id IS NOT NEW.id OR OLD.user_id IS NOT NEW.user_id OR OLD.room_id IS NOT NEW.room_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id = OLD.user_id) OR (id = NEW.user_id);
END;
CREATE TRIGGER activity_revision_messages_insert
AFTER INSERT ON messages
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Message' AND ai.source_id = NEW.id) OR (ai.source_type = 'SavedItem' AND ai.source_id IN (SELECT id FROM saved_items WHERE message_id = NEW.id)));
END;
CREATE TRIGGER activity_revision_messages_delete
BEFORE DELETE ON messages
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Message' AND ai.source_id = OLD.id) OR (ai.source_type = 'SavedItem' AND ai.source_id IN (SELECT id FROM saved_items WHERE message_id = OLD.id)));
END;
CREATE TRIGGER activity_revision_messages_update
AFTER UPDATE OF id, room_id ON messages
WHEN OLD.id IS NOT NEW.id OR OLD.room_id IS NOT NEW.room_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Message' AND ai.source_id = OLD.id) OR (ai.source_type = 'SavedItem' AND ai.source_id IN (SELECT id FROM saved_items WHERE message_id = OLD.id)))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Message' AND ai.source_id = NEW.id) OR (ai.source_type = 'SavedItem' AND ai.source_id IN (SELECT id FROM saved_items WHERE message_id = NEW.id))));
END;
CREATE TRIGGER activity_revision_saved_items_insert
AFTER INSERT ON saved_items
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'SavedItem' AND ai.source_id = NEW.id));
END;
CREATE TRIGGER activity_revision_saved_items_delete
BEFORE DELETE ON saved_items
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'SavedItem' AND ai.source_id = OLD.id));
END;
CREATE TRIGGER activity_revision_saved_items_update
AFTER UPDATE OF id, message_id ON saved_items
WHEN OLD.id IS NOT NEW.id OR OLD.message_id IS NOT NEW.message_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'SavedItem' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'SavedItem' AND ai.source_id = NEW.id)));
END;
CREATE TRIGGER activity_revision_work_thread_events_insert
AFTER INSERT ON work_thread_events
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'WorkThreadEvent' AND ai.source_id = NEW.id));
END;
CREATE TRIGGER activity_revision_work_thread_events_delete
BEFORE DELETE ON work_thread_events
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'WorkThreadEvent' AND ai.source_id = OLD.id));
END;
CREATE TRIGGER activity_revision_work_thread_events_update
AFTER UPDATE OF id, channel_thread_id ON work_thread_events
WHEN OLD.id IS NOT NEW.id OR OLD.channel_thread_id IS NOT NEW.channel_thread_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'WorkThreadEvent' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'WorkThreadEvent' AND ai.source_id = NEW.id)));
END;
CREATE TRIGGER activity_revision_channel_threads_insert
AFTER INSERT ON channel_threads
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE ai.source_type = 'WorkThreadEvent' AND ai.source_id IN (SELECT id FROM work_thread_events WHERE channel_thread_id = NEW.id));
END;
CREATE TRIGGER activity_revision_channel_threads_delete
BEFORE DELETE ON channel_threads
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE ai.source_type = 'WorkThreadEvent' AND ai.source_id IN (SELECT id FROM work_thread_events WHERE channel_thread_id = OLD.id));
END;
CREATE TRIGGER activity_revision_channel_threads_update
AFTER UPDATE OF id, room_id ON channel_threads
WHEN OLD.id IS NOT NEW.id OR OLD.room_id IS NOT NEW.room_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE ai.source_type = 'WorkThreadEvent' AND ai.source_id IN (SELECT id FROM work_thread_events WHERE channel_thread_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE ai.source_type = 'WorkThreadEvent' AND ai.source_id IN (SELECT id FROM work_thread_events WHERE channel_thread_id = NEW.id)));
END;
CREATE TRIGGER activity_revision_board_sla_nudges_insert
AFTER INSERT ON board_sla_nudges
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'BoardSlaNudge' AND ai.source_id = NEW.id));
END;
CREATE TRIGGER activity_revision_board_sla_nudges_delete
BEFORE DELETE ON board_sla_nudges
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'BoardSlaNudge' AND ai.source_id = OLD.id));
END;
CREATE TRIGGER activity_revision_board_sla_nudges_update
AFTER UPDATE OF id, room_id ON board_sla_nudges
WHEN OLD.id IS NOT NEW.id OR OLD.room_id IS NOT NEW.room_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'BoardSlaNudge' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'BoardSlaNudge' AND ai.source_id = NEW.id)));
END;
CREATE TRIGGER activity_revision_huddle_grants_insert
AFTER INSERT ON huddle_grants
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'HuddleGrant' AND ai.source_id = NEW.id));
END;
CREATE TRIGGER activity_revision_huddle_grants_delete
BEFORE DELETE ON huddle_grants
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'HuddleGrant' AND ai.source_id = OLD.id));
END;
CREATE TRIGGER activity_revision_huddle_grants_update
AFTER UPDATE OF id, room_id ON huddle_grants
WHEN OLD.id IS NOT NEW.id OR OLD.room_id IS NOT NEW.room_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'HuddleGrant' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'HuddleGrant' AND ai.source_id = NEW.id)));
END;
CREATE TRIGGER activity_revision_events_insert
AFTER INSERT ON events
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Event' AND ai.source_id = NEW.id));
END;
CREATE TRIGGER activity_revision_events_delete
BEFORE DELETE ON events
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Event' AND ai.source_id = OLD.id));
END;
CREATE TRIGGER activity_revision_events_update
AFTER UPDATE OF id, room_id ON events
WHEN OLD.id IS NOT NEW.id OR OLD.room_id IS NOT NEW.room_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Event' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Event' AND ai.source_id = NEW.id)));
END;
CREATE TRIGGER activity_revision_agent_approvals_insert
AFTER INSERT ON agent_approvals
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentApproval' AND ai.source_id = NEW.id));
END;
CREATE TRIGGER activity_revision_agent_approvals_delete
BEFORE DELETE ON agent_approvals
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentApproval' AND ai.source_id = OLD.id));
END;
CREATE TRIGGER activity_revision_agent_approvals_update
AFTER UPDATE OF id, agent_id ON agent_approvals
WHEN OLD.id IS NOT NEW.id OR OLD.agent_id IS NOT NEW.agent_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentApproval' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentApproval' AND ai.source_id = NEW.id)));
END;
CREATE TRIGGER activity_revision_agent_budget_notices_insert
AFTER INSERT ON agent_budget_notices
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentBudgetNotice' AND ai.source_id = NEW.id));
END;
CREATE TRIGGER activity_revision_agent_budget_notices_delete
BEFORE DELETE ON agent_budget_notices
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentBudgetNotice' AND ai.source_id = OLD.id));
END;
CREATE TRIGGER activity_revision_agent_budget_notices_update
AFTER UPDATE OF id, agent_id ON agent_budget_notices
WHEN OLD.id IS NOT NEW.id OR OLD.agent_id IS NOT NEW.agent_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentBudgetNotice' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentBudgetNotice' AND ai.source_id = NEW.id)));
END;
CREATE TRIGGER activity_revision_scheduled_messages_insert
AFTER INSERT ON scheduled_messages
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'ScheduledMessage' AND ai.source_id = NEW.id));
END;
CREATE TRIGGER activity_revision_scheduled_messages_delete
BEFORE DELETE ON scheduled_messages
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'ScheduledMessage' AND ai.source_id = OLD.id));
END;
CREATE TRIGGER activity_revision_scheduled_messages_update
AFTER UPDATE OF id, user_id ON scheduled_messages
WHEN OLD.id IS NOT NEW.id OR OLD.user_id IS NOT NEW.user_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'ScheduledMessage' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'ScheduledMessage' AND ai.source_id = NEW.id)));
END;
CREATE TRIGGER activity_revision_two_factor_credentials_insert
AFTER INSERT ON two_factor_credentials
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'TwoFactorCredential' AND ai.source_id = NEW.id));
END;
CREATE TRIGGER activity_revision_two_factor_credentials_delete
BEFORE DELETE ON two_factor_credentials
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'TwoFactorCredential' AND ai.source_id = OLD.id));
END;
CREATE TRIGGER activity_revision_two_factor_credentials_update
AFTER UPDATE OF id, user_id ON two_factor_credentials
WHEN OLD.id IS NOT NEW.id OR OLD.user_id IS NOT NEW.user_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'TwoFactorCredential' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'TwoFactorCredential' AND ai.source_id = NEW.id)));
END;
CREATE TRIGGER activity_revision_sessions_insert
AFTER INSERT ON sessions
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Session' AND ai.source_id = NEW.id));
END;
CREATE TRIGGER activity_revision_sessions_delete
BEFORE DELETE ON sessions
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Session' AND ai.source_id = OLD.id));
END;
CREATE TRIGGER activity_revision_sessions_update
AFTER UPDATE OF id, user_id ON sessions
WHEN OLD.id IS NOT NEW.id OR OLD.user_id IS NOT NEW.user_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Session' AND ai.source_id = OLD.id))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'Session' AND ai.source_id = NEW.id)));
END;
CREATE TRIGGER activity_revision_agents_insert
AFTER INSERT ON agents
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentApproval' AND ai.source_id IN (SELECT id FROM agent_approvals WHERE agent_id = NEW.id)) OR (ai.source_type = 'AgentBudgetNotice' AND ai.source_id IN (SELECT id FROM agent_budget_notices WHERE agent_id = NEW.id)));
END;
CREATE TRIGGER activity_revision_agents_delete
BEFORE DELETE ON agents
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentApproval' AND ai.source_id IN (SELECT id FROM agent_approvals WHERE agent_id = OLD.id)) OR (ai.source_type = 'AgentBudgetNotice' AND ai.source_id IN (SELECT id FROM agent_budget_notices WHERE agent_id = OLD.id)));
END;
CREATE TRIGGER activity_revision_agents_update
AFTER UPDATE OF id, owner_id, user_id ON agents
WHEN OLD.id IS NOT NEW.id OR OLD.owner_id IS NOT NEW.owner_id OR OLD.user_id IS NOT NEW.user_id
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentApproval' AND ai.source_id IN (SELECT id FROM agent_approvals WHERE agent_id = OLD.id)) OR (ai.source_type = 'AgentBudgetNotice' AND ai.source_id IN (SELECT id FROM agent_budget_notices WHERE agent_id = OLD.id)))) OR (id IN (SELECT ai.user_id FROM activity_items ai WHERE (ai.source_type = 'AgentApproval' AND ai.source_id IN (SELECT id FROM agent_approvals WHERE agent_id = NEW.id)) OR (ai.source_type = 'AgentBudgetNotice' AND ai.source_id IN (SELECT id FROM agent_budget_notices WHERE agent_id = NEW.id))));
END;
CREATE TRIGGER activity_revision_users_insert
AFTER INSERT ON users
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id = NEW.id OR id IN (SELECT ai.user_id FROM activity_items ai WHERE ai.source_type = 'AgentApproval' AND ai.source_id IN (SELECT ap.id FROM agent_approvals ap JOIN agents ag ON ag.id = ap.agent_id WHERE ag.user_id = NEW.id));
END;
CREATE TRIGGER activity_revision_users_delete
BEFORE DELETE ON users
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE id = OLD.id OR id IN (SELECT ai.user_id FROM activity_items ai WHERE ai.source_type = 'AgentApproval' AND ai.source_id IN (SELECT ap.id FROM agent_approvals ap JOIN agents ag ON ag.id = ap.agent_id WHERE ag.user_id = OLD.id));
END;
CREATE TRIGGER activity_revision_users_update
AFTER UPDATE OF id, status, role ON users
WHEN OLD.id IS NOT NEW.id OR OLD.status IS NOT NEW.status OR OLD.role IS NOT NEW.role
BEGIN
    UPDATE users SET activity_revision = activity_revision + 1
    WHERE (id = OLD.id OR id IN (SELECT ai.user_id FROM activity_items ai WHERE ai.source_type = 'AgentApproval' AND ai.source_id IN (SELECT ap.id FROM agent_approvals ap JOIN agents ag ON ag.id = ap.agent_id WHERE ag.user_id = OLD.id))) OR (id = NEW.id OR id IN (SELECT ai.user_id FROM activity_items ai WHERE ai.source_type = 'AgentApproval' AND ai.source_id IN (SELECT ap.id FROM agent_approvals ap JOIN agents ag ON ag.id = ap.agent_id WHERE ag.user_id = NEW.id)));
END;
CREATE UNIQUE INDEX "index_rooms_on_creator_id_and_client_room_id" ON "rooms" ("creator_id", "client_room_id") WHERE client_room_id IS NOT NULL;
CREATE UNIQUE INDEX "index_channel_threads_on_room_creator_client_post_id" ON "channel_threads" ("room_id", "creator_id", "client_post_id") WHERE client_post_id IS NOT NULL;
CREATE INDEX "index_activity_items_on_unread_message_pings" ON "activity_items" ("user_id", "event_type") WHERE read_at IS NULL AND source_type = 'Message';
