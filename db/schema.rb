# This file is auto-generated from the current state of the database. Instead
# of editing this file, please use the migrations feature of Active Record to
# incrementally modify your database, and then regenerate this schema definition.
#
# This file is the source Rails uses to define your schema when running `bin/rails
# db:schema:load`. When creating a new database, `bin/rails db:schema:load` tends to
# be faster and is potentially less error prone than running all of your
# migrations from scratch. Old migrations may fail to apply correctly if those
# migrations use external dependencies or application code.
#
# It's strongly recommended that you check this file into your version control system.

ActiveRecord::Schema[8.2].define(version: 2026_09_23_054816) do
  create_table "accounts", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.text "custom_styles"
    t.string "join_code", null: false
    t.string "name", null: false
    t.json "settings"
    t.integer "singleton_guard", default: 0, null: false
    t.datetime "updated_at", null: false
    t.index ["singleton_guard"], name: "index_accounts_on_singleton_guard", unique: true
  end

  create_table "action_mailbox_inbound_emails", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.string "message_checksum", null: false
    t.string "message_id", null: false
    t.integer "status", default: 0, null: false
    t.datetime "updated_at", null: false
    t.index ["message_id", "message_checksum"], name: "index_action_mailbox_inbound_emails_uniqueness", unique: true
  end

  create_table "action_text_rich_texts", force: :cascade do |t|
    t.text "body"
    t.datetime "created_at", null: false
    t.string "name", null: false
    t.bigint "record_id", null: false
    t.string "record_type", null: false
    t.datetime "updated_at", null: false
    t.index ["record_type", "record_id", "name"], name: "index_action_text_rich_texts_uniqueness", unique: true
  end

  create_table "active_storage_attachments", force: :cascade do |t|
    t.bigint "blob_id", null: false
    t.datetime "created_at", null: false
    t.string "name", null: false
    t.bigint "record_id", null: false
    t.string "record_type", null: false
    t.index ["blob_id"], name: "index_active_storage_attachments_on_blob_id"
    t.index ["record_type", "record_id", "name", "blob_id"], name: "index_active_storage_attachments_uniqueness", unique: true
  end

  create_table "active_storage_blobs", force: :cascade do |t|
    t.bigint "byte_size", null: false
    t.string "checksum"
    t.string "content_type"
    t.datetime "created_at", null: false
    t.string "filename", null: false
    t.string "key", null: false
    t.text "metadata"
    t.string "service_name", null: false
    t.index ["key"], name: "index_active_storage_blobs_on_key", unique: true
  end

  create_table "active_storage_variant_records", force: :cascade do |t|
    t.bigint "blob_id", null: false
    t.string "variation_digest", null: false
    t.index ["blob_id", "variation_digest"], name: "index_active_storage_variant_records_uniqueness", unique: true
  end

  create_table "activity_items", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.string "event_type", null: false
    t.datetime "handled_at"
    t.datetime "read_at"
    t.integer "source_id", null: false
    t.string "source_type", null: false
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["event_type", "created_at"], name: "index_activity_items_on_event_type_and_created_at"
    t.index ["source_type", "source_id"], name: "index_activity_items_on_source"
    t.index ["user_id", "read_at", "handled_at", "created_at"], name: "index_activity_items_on_user_and_state"
    t.index ["user_id", "source_type", "source_id"], name: "index_activity_items_on_user_and_source", unique: true
    t.index ["user_id", "updated_at"], name: "index_activity_items_on_user_id_and_updated_at"
  end

  create_table "agent_approvals", force: :cascade do |t|
    t.string "action", null: false
    t.integer "agent_credential_id"
    t.integer "agent_id", null: false
    t.datetime "created_at", null: false
    t.datetime "decided_at"
    t.integer "decided_by_id"
    t.string "decision_note"
    t.datetime "expires_at", null: false
    t.string "external_id"
    t.integer "fizzy_connected_account_id"
    t.string "fizzy_user_id"
    t.string "fizzy_user_name"
    t.integer "github_account_id"
    t.string "github_login"
    t.text "payload"
    t.integer "room_id"
    t.string "status", default: "pending", null: false
    t.text "summary", null: false
    t.datetime "updated_at", null: false
    t.index ["agent_id", "external_id"], name: "index_agent_approvals_on_agent_id_and_external_id", unique: true, where: "external_id IS NOT NULL"
    t.index ["agent_id", "status"], name: "index_agent_approvals_on_agent_id_and_status"
    t.index ["fizzy_connected_account_id"], name: "index_agent_approvals_on_fizzy_connected_account_id"
  end

  create_table "agent_credentials", force: :cascade do |t|
    t.integer "agent_id", null: false
    t.datetime "created_at", null: false
    t.integer "created_by_id", null: false
    t.datetime "expires_at"
    t.datetime "last_used_at"
    t.string "last_used_ip"
    t.string "name", null: false
    t.datetime "revoked_at"
    t.string "token_digest", null: false
    t.string "token_last_four", null: false
    t.datetime "updated_at", null: false
    t.index ["agent_id", "revoked_at"], name: "index_agent_credentials_on_agent_id_and_revoked_at"
    t.index ["token_digest"], name: "index_agent_credentials_on_token_digest", unique: true
  end

  create_table "agent_events", force: :cascade do |t|
    t.integer "actor_id"
    t.integer "agent_approval_id"
    t.integer "agent_credential_id"
    t.integer "agent_id", null: false
    t.string "chain_id"
    t.datetime "created_at", null: false
    t.string "detail"
    t.string "event_type", null: false
    t.integer "hop", default: 0, null: false
    t.integer "message_id"
    t.json "metadata"
    t.string "outcome"
    t.integer "room_id"
    t.integer "webhook_attempts", default: 0, null: false
    t.text "webhook_last_error"
    t.datetime "webhook_next_attempt_at"
    t.string "webhook_status", default: "none", null: false
    t.index ["agent_id", "agent_approval_id"], name: "index_agent_events_on_agent_fizzy_approval", unique: true, where: "event_type = 'fizzy_action_completed' AND agent_approval_id IS NOT NULL"
    t.index ["agent_id", "agent_approval_id"], name: "index_agent_events_on_agent_github_approval", unique: true, where: "event_type = 'github_action_completed' AND agent_approval_id IS NOT NULL"
    t.index ["agent_id", "created_at"], name: "index_agent_events_on_agent_id_and_created_at"
    t.index ["agent_id", "outcome", "id"], name: "index_agent_events_on_agent_outcome_id"
    t.index ["webhook_status", "webhook_next_attempt_at"], name: "index_agent_events_on_webhook_recovery"
  end

  create_table "agent_grants", force: :cascade do |t|
    t.integer "agent_id", null: false
    t.string "capability", null: false
    t.datetime "created_at", null: false
    t.integer "granted_by_id", null: false
    t.datetime "revoked_at"
    t.integer "room_id"
    t.datetime "updated_at", null: false
    t.index ["agent_id", "capability"], name: "index_agent_grants_on_agent_capability_active_workspace", unique: true, where: "revoked_at IS NULL AND room_id IS NULL"
    t.index ["agent_id", "revoked_at"], name: "index_agent_grants_on_agent_id_and_revoked_at"
    t.index ["agent_id", "room_id", "capability"], name: "index_agent_grants_on_agent_room_capability_active", unique: true, where: "revoked_at IS NULL AND room_id IS NOT NULL"
  end

  create_table "agent_slash_commands", force: :cascade do |t|
    t.integer "agent_id", null: false
    t.datetime "created_at", null: false
    t.string "description"
    t.string "name", null: false
    t.integer "room_id", null: false
    t.datetime "updated_at", null: false
    t.index ["agent_id"], name: "index_agent_slash_commands_on_agent_id"
    t.index ["room_id", "name"], name: "index_agent_slash_commands_on_room_id_and_name", unique: true
    t.index ["room_id"], name: "index_agent_slash_commands_on_room_id"
  end

  create_table "agents", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.text "description"
    t.string "kind", default: "personal", null: false
    t.datetime "last_seen_at"
    t.integer "owner_id"
    t.string "provider"
    t.string "runtime"
    t.string "status", default: "idle", null: false
    t.datetime "status_changed_at"
    t.string "status_note"
    t.datetime "suspended_at"
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.string "webhook_signing_secret"
    t.index ["owner_id", "kind"], name: "index_agents_on_owner_id_and_kind"
    t.index ["user_id"], name: "index_agents_on_user_id", unique: true
  end

  create_table "bans", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.string "ip_address", null: false
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["ip_address"], name: "index_bans_on_ip_address"
    t.index ["user_id"], name: "index_bans_on_user_id"
  end

  create_table "boosts", force: :cascade do |t|
    t.integer "booster_id", null: false
    t.string "content", limit: 16, null: false
    t.datetime "created_at", null: false
    t.integer "message_id", null: false
    t.datetime "updated_at", null: false
    t.index ["booster_id"], name: "index_boosts_on_booster_id"
    t.index ["message_id"], name: "index_boosts_on_message_id"
  end

  create_table "calendar_push_channels", force: :cascade do |t|
    t.string "channel_id", null: false
    t.datetime "created_at", null: false
    t.datetime "expires_at"
    t.string "last_error"
    t.bigint "last_message_number", default: 0, null: false
    t.datetime "last_notification_at"
    t.string "resource_id"
    t.string "token_digest", null: false
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["channel_id"], name: "index_calendar_push_channels_on_channel_id", unique: true
    t.index ["user_id"], name: "index_calendar_push_channels_on_user_id", unique: true
  end

  create_table "channel_threads", force: :cascade do |t|
    t.integer "auto_archive_after_minutes", default: 4320, null: false
    t.datetime "closed_at"
    t.datetime "created_at", null: false
    t.integer "creator_id", null: false
    t.datetime "last_activity_at", null: false
    t.datetime "locked_at"
    t.string "name", null: false
    t.integer "parent_message_id"
    t.text "result_markdown"
    t.datetime "result_updated_at"
    t.integer "result_updated_by_id"
    t.integer "room_id", null: false
    t.string "run_url"
    t.datetime "updated_at", null: false
    t.integer "work_owner_id"
    t.string "work_status"
    t.index ["creator_id"], name: "index_channel_threads_on_creator_id"
    t.index ["parent_message_id"], name: "index_channel_threads_on_parent_message_id", unique: true, where: "parent_message_id IS NOT NULL"
    t.index ["room_id", "closed_at", "locked_at"], name: "index_channel_threads_on_room_id_and_closed_at_and_locked_at"
    t.index ["room_id", "last_activity_at"], name: "index_channel_threads_on_room_id_and_last_activity_at"
    t.index ["room_id", "work_status", "last_activity_at"], name: "index_channel_threads_on_room_and_work_status_and_activity"
    t.index ["work_owner_id"], name: "index_channel_threads_on_work_owner_id"
  end

  create_table "dnd_allowed_users", force: :cascade do |t|
    t.integer "allowed_user_id", null: false
    t.datetime "created_at", null: false
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["allowed_user_id"], name: "index_dnd_allowed_users_on_allowed_user_id"
    t.index ["user_id", "allowed_user_id"], name: "index_dnd_allowed_users_on_user_id_and_allowed_user_id", unique: true
    t.index ["user_id"], name: "index_dnd_allowed_users_on_user_id"
  end

  create_table "drive_attachments", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.string "file_id", null: false
    t.integer "message_id", null: false
    t.index ["message_id", "file_id"], name: "index_drive_attachments_on_message_id_and_file_id", unique: true
    t.index ["message_id"], name: "index_drive_attachments_on_message_id"
  end

  create_table "event_attendances", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.integer "event_id", null: false
    t.string "response", null: false
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["event_id", "user_id"], name: "index_event_attendances_on_event_id_and_user_id", unique: true
  end

  create_table "event_calendar_entries", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.integer "event_id", null: false
    t.string "google_event_id", null: false
    t.string "last_error"
    t.datetime "synced_at"
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["event_id", "user_id"], name: "index_event_calendar_entries_on_event_id_and_user_id", unique: true
    t.index ["event_id"], name: "index_event_calendar_entries_on_event_id"
    t.index ["user_id"], name: "index_event_calendar_entries_on_user_id"
  end

  create_table "event_references", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.integer "event_id", null: false
    t.integer "message_id", null: false
    t.datetime "updated_at", null: false
    t.index ["event_id"], name: "index_event_references_on_event_id"
    t.index ["message_id", "event_id"], name: "index_event_references_on_message_id_and_event_id", unique: true
    t.index ["message_id"], name: "index_event_references_on_message_id"
  end

  create_table "events", force: :cascade do |t|
    t.datetime "cancelled_at"
    t.datetime "created_at", null: false
    t.text "description"
    t.datetime "ends_at"
    t.string "meet_link"
    t.boolean "meet_link_requested", default: false, null: false
    t.integer "organizer_id", null: false
    t.string "recurrence_rule"
    t.date "recurrence_until"
    t.datetime "reminded_at"
    t.integer "room_id", null: false
    t.integer "series_id"
    t.datetime "starts_at", null: false
    t.string "time_zone", null: false
    t.string "title", null: false
    t.datetime "updated_at", null: false
    t.integer "venue_room_id"
    t.index ["organizer_id"], name: "index_events_on_organizer_id"
    t.index ["reminded_at", "starts_at"], name: "index_events_on_reminded_starts"
    t.index ["room_id", "starts_at"], name: "index_events_on_room_id_and_starts_at"
    t.index ["series_id", "starts_at"], name: "index_events_on_series_slot", unique: true, where: "series_id IS NOT NULL AND cancelled_at IS NULL"
    t.index ["series_id"], name: "index_events_on_series_id"
    t.index ["venue_room_id"], name: "index_events_on_venue_room_id"
  end

  create_table "fizzy_card_caches", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.string "fetch_error"
    t.datetime "fetch_requested_at"
    t.datetime "fetched_at"
    t.integer "fizzy_card_id", null: false
    t.json "payload"
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["fizzy_card_id", "user_id"], name: "index_fizzy_card_caches_on_card_and_user", unique: true
    t.index ["fizzy_card_id"], name: "index_fizzy_card_caches_on_fizzy_card_id"
    t.index ["user_id"], name: "index_fizzy_card_caches_on_user_id"
  end

  create_table "fizzy_card_references", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.integer "fizzy_card_id", null: false
    t.integer "message_id", null: false
    t.datetime "updated_at", null: false
    t.index ["fizzy_card_id"], name: "index_fizzy_card_references_on_fizzy_card_id"
    t.index ["message_id", "fizzy_card_id"], name: "index_fizzy_card_refs_on_message_and_card", unique: true
    t.index ["message_id"], name: "index_fizzy_card_references_on_message_id"
  end

  create_table "fizzy_cards", force: :cascade do |t|
    t.string "account_id", null: false
    t.datetime "created_at", null: false
    t.integer "number", null: false
    t.datetime "updated_at", null: false
    t.index ["account_id", "number"], name: "index_fizzy_cards_on_account_id_and_number", unique: true
  end

  create_table "fizzy_connected_accounts", force: :cascade do |t|
    t.string "access_token", null: false
    t.datetime "created_at", null: false
    t.string "disconnected_reason"
    t.string "fizzy_account_id", null: false
    t.string "fizzy_account_name"
    t.string "fizzy_user_id"
    t.string "fizzy_user_name"
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["user_id"], name: "index_fizzy_connected_accounts_on_user_id", unique: true
  end

  create_table "github_connected_accounts", force: :cascade do |t|
    t.string "access_token", null: false
    t.datetime "created_at", null: false
    t.string "disconnected_reason"
    t.string "github_login", null: false
    t.string "last_error"
    t.string "refresh_token"
    t.datetime "token_expires_at"
    t.string "token_source", default: "pat", null: false
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["user_id"], name: "index_github_connected_accounts_on_user_id", unique: true
  end

  create_table "github_notifications", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.string "dedupe_key", null: false
    t.integer "message_id"
    t.integer "subscription_id", null: false
    t.datetime "updated_at", null: false
    t.index ["message_id"], name: "index_github_notifications_on_message_id"
    t.index ["subscription_id", "dedupe_key"], name: "index_github_notifications_on_subscription_and_key", unique: true
    t.index ["subscription_id"], name: "index_github_notifications_on_subscription_id"
  end

  create_table "github_pull_request_references", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.integer "github_pull_request_id", null: false
    t.integer "message_id", null: false
    t.datetime "updated_at", null: false
    t.index ["github_pull_request_id"], name: "index_github_pull_request_references_on_github_pull_request_id"
    t.index ["message_id", "github_pull_request_id"], name: "index_gh_pr_refs_on_message_and_pr", unique: true
    t.index ["message_id"], name: "index_github_pull_request_references_on_message_id"
  end

  create_table "github_pull_request_threads", force: :cascade do |t|
    t.integer "channel_thread_id", null: false
    t.datetime "created_at", null: false
    t.integer "github_pull_request_id", null: false
    t.integer "room_id", null: false
    t.datetime "updated_at", null: false
    t.index ["channel_thread_id"], name: "index_github_pr_threads_on_thread", unique: true
    t.index ["channel_thread_id"], name: "index_github_pull_request_threads_on_channel_thread_id"
    t.index ["github_pull_request_id", "room_id"], name: "index_github_pr_threads_on_pr_and_room", unique: true
    t.index ["github_pull_request_id"], name: "index_github_pull_request_threads_on_github_pull_request_id"
    t.index ["room_id"], name: "index_github_pull_request_threads_on_room_id"
  end

  create_table "github_pull_requests", force: :cascade do |t|
    t.string "author_avatar_url"
    t.string "author_login"
    t.string "base_branch"
    t.text "changed_files"
    t.datetime "changed_files_fetched_at"
    t.string "check_status"
    t.datetime "created_at", null: false
    t.string "fetch_error"
    t.datetime "fetch_requested_at"
    t.datetime "fetched_at"
    t.datetime "github_updated_at"
    t.string "head_branch"
    t.string "head_sha"
    t.string "html_url"
    t.integer "number", null: false
    t.string "owner", null: false
    t.json "payload"
    t.boolean "private"
    t.string "repo", null: false
    t.string "review_decision"
    t.string "state"
    t.string "title"
    t.datetime "updated_at", null: false
    t.index ["owner", "repo", "number"], name: "index_github_pull_requests_on_owner_repo_number", unique: true
  end

  create_table "github_repository_subscriptions", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.integer "created_by_id"
    t.json "events", default: [], null: false
    t.string "owner", null: false
    t.boolean "reader_verified", default: false, null: false
    t.string "repo", null: false
    t.integer "room_id", null: false
    t.datetime "updated_at", null: false
    t.index ["created_by_id"], name: "index_github_repository_subscriptions_on_created_by_id"
    t.index ["owner", "repo"], name: "index_github_subscriptions_on_owner_and_repo"
    t.index ["room_id", "owner", "repo"], name: "index_github_subscriptions_on_room_and_repo", unique: true
    t.index ["room_id"], name: "index_github_repository_subscriptions_on_room_id"
  end

  create_table "github_webhook_deliveries", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.string "delivery_guid", null: false
    t.string "event"
    t.datetime "updated_at", null: false
    t.index ["delivery_guid"], name: "index_github_webhook_deliveries_on_delivery_guid", unique: true
  end

  create_table "google_accounts", force: :cascade do |t|
    t.string "access_token"
    t.datetime "access_token_expires_at"
    t.datetime "created_at", null: false
    t.string "disconnected_reason"
    t.string "email", null: false
    t.string "refresh_token"
    t.string "scopes"
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["user_id"], name: "index_google_accounts_on_user_id", unique: true
  end

  create_table "google_identities", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.string "domain"
    t.string "email", null: false
    t.string "subject", null: false
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["subject"], name: "index_google_identities_on_subject", unique: true
    t.index ["user_id"], name: "index_google_identities_on_user_id", unique: true
  end

  create_table "huddle_cleanups", force: :cascade do |t|
    t.integer "attempts", default: 0, null: false
    t.datetime "completed_at"
    t.datetime "created_at", null: false
    t.datetime "enqueued_at"
    t.integer "huddle_grant_id"
    t.string "identity"
    t.datetime "last_attempted_at"
    t.datetime "next_attempt_at"
    t.string "operation", null: false
    t.string "room_name", null: false
    t.datetime "updated_at", null: false
    t.index ["completed_at", "next_attempt_at"], name: "index_huddle_cleanups_on_completed_at_and_next_attempt_at"
    t.index ["huddle_grant_id"], name: "index_huddle_cleanups_on_huddle_grant_id"
    t.index ["operation", "huddle_grant_id"], name: "index_huddle_cleanups_on_unique_participant_removal", unique: true, where: "operation = 'remove_participant'"
    t.index ["operation", "room_name"], name: "index_huddle_cleanups_on_unique_room_deletion", unique: true, where: "operation = 'delete_room'"
  end

  create_table "huddle_grants", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.string "identity", null: false
    t.datetime "last_issued_at"
    t.datetime "last_seen_at"
    t.integer "membership_id", null: false
    t.datetime "revoked_at"
    t.integer "room_id", null: false
    t.string "room_name", null: false
    t.integer "session_id", null: false
    t.string "stage_role"
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["identity"], name: "index_huddle_grants_on_identity", unique: true
    t.index ["membership_id"], name: "index_huddle_grants_on_membership_id"
    t.index ["room_id", "last_seen_at"], name: "index_huddle_grants_on_room_and_last_seen_at"
    t.index ["room_id"], name: "index_huddle_grants_on_room_id"
    t.index ["session_id", "membership_id"], name: "index_active_huddle_grants_on_session_and_membership", unique: true, where: "revoked_at IS NULL"
    t.index ["session_id"], name: "index_huddle_grants_on_session_id"
    t.index ["user_id"], name: "index_huddle_grants_on_user_id"
  end

  create_table "keyword_alerts", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.string "phrase", null: false
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["user_id", "phrase"], name: "index_keyword_alerts_on_user_id_and_phrase", unique: true
    t.index ["user_id"], name: "index_keyword_alerts_on_user_id"
  end

  create_table "link_embed_references", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.integer "link_embed_id", null: false
    t.integer "message_id", null: false
    t.integer "position", default: 0, null: false
    t.datetime "updated_at", null: false
    t.string "url"
    t.index ["link_embed_id"], name: "index_link_embed_references_on_link_embed_id"
    t.index ["message_id", "link_embed_id"], name: "index_link_embed_references_on_message_and_embed", unique: true
    t.index ["message_id"], name: "index_link_embed_references_on_message_id"
  end

  create_table "link_embeds", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.text "description"
    t.datetime "expires_at"
    t.string "fetch_error"
    t.datetime "fetch_requested_at"
    t.datetime "fetched_at"
    t.string "image_url"
    t.string "normalized_url", null: false
    t.string "site_name"
    t.string "title"
    t.datetime "updated_at", null: false
    t.index ["normalized_url"], name: "index_link_embeds_on_normalized_url", unique: true
  end

  create_table "memberships", force: :cascade do |t|
    t.datetime "connected_at"
    t.integer "connections", default: 0, null: false
    t.datetime "created_at", null: false
    t.datetime "hand_raised_at"
    t.string "involvement", default: "mentions"
    t.integer "room_id", null: false
    t.string "stage_role"
    t.datetime "unread_at"
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["room_id", "created_at"], name: "index_memberships_on_room_id_and_created_at"
    t.index ["room_id", "stage_role"], name: "index_memberships_on_room_id_and_stage_role"
    t.index ["room_id", "user_id"], name: "index_memberships_on_room_id_and_user_id", unique: true
    t.index ["room_id"], name: "index_memberships_on_room_id"
    t.index ["user_id"], name: "index_memberships_on_user_id"
  end

  create_table "message_pins", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.integer "message_id", null: false
    t.integer "pinner_id", null: false
    t.integer "room_id", null: false
    t.datetime "updated_at", null: false
    t.index ["message_id"], name: "index_message_pins_on_message_id", unique: true
    t.index ["pinner_id"], name: "index_message_pins_on_pinner_id"
    t.index ["room_id", "created_at"], name: "index_message_pins_on_room_id_and_created_at"
    t.index ["room_id"], name: "index_message_pins_on_room_id"
  end

  create_table "messages", force: :cascade do |t|
    t.boolean "action", default: false, null: false
    t.string "client_message_id", null: false
    t.datetime "created_at", null: false
    t.integer "creator_id", null: false
    t.datetime "edited_at"
    t.boolean "embeds_suppressed", default: false, null: false
    t.text "forward_note"
    t.datetime "forwarded_at"
    t.integer "forwarded_from_message_id"
    t.boolean "forwarded_markdown", default: false, null: false
    t.text "markdown_source"
    t.boolean "reply_notify_author", default: true, null: false
    t.datetime "reply_target_deleted_at"
    t.integer "reply_to_message_id"
    t.integer "room_id", null: false
    t.boolean "system_note", default: false, null: false
    t.integer "thread_id"
    t.datetime "updated_at", null: false
    t.index ["creator_id"], name: "index_messages_on_creator_id"
    t.index ["forwarded_from_message_id"], name: "index_messages_on_forwarded_from_message_id"
    t.index ["reply_to_message_id"], name: "index_messages_on_reply_to_message_id"
    t.index ["room_id", "creator_id", "client_message_id"], name: "index_messages_on_room_creator_client_id"
    t.index ["room_id", "thread_id", "created_at"], name: "index_messages_on_room_thread_created"
    t.index ["room_id"], name: "index_messages_on_room_id"
    t.index ["thread_id", "created_at"], name: "index_messages_on_thread_created"
    t.index ["thread_id"], name: "index_messages_on_thread_id"
  end

  create_table "poll_options", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.string "label", null: false
    t.integer "poll_id", null: false
    t.integer "position", default: 0, null: false
    t.datetime "updated_at", null: false
    t.index ["poll_id", "position"], name: "index_poll_options_on_poll_id_and_position"
    t.index ["poll_id"], name: "index_poll_options_on_poll_id"
  end

  create_table "poll_votes", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.integer "poll_id", null: false
    t.integer "poll_option_id", null: false
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["poll_id", "user_id"], name: "index_poll_votes_on_poll_id_and_user_id"
    t.index ["poll_id"], name: "index_poll_votes_on_poll_id"
    t.index ["poll_option_id", "user_id"], name: "index_poll_votes_on_poll_option_id_and_user_id", unique: true
    t.index ["poll_option_id"], name: "index_poll_votes_on_poll_option_id"
    t.index ["user_id"], name: "index_poll_votes_on_user_id"
  end

  create_table "polls", force: :cascade do |t|
    t.boolean "anonymous", default: false, null: false
    t.datetime "closed_at"
    t.datetime "closes_at"
    t.datetime "created_at", null: false
    t.integer "message_id", null: false
    t.boolean "multiple", default: false, null: false
    t.datetime "updated_at", null: false
    t.index ["message_id"], name: "index_polls_on_message_id", unique: true
  end

  create_table "push_subscriptions", force: :cascade do |t|
    t.string "auth_key"
    t.datetime "created_at", null: false
    t.string "endpoint"
    t.string "p256dh_key"
    t.datetime "updated_at", null: false
    t.string "user_agent"
    t.integer "user_id", null: false
    t.index ["endpoint", "p256dh_key", "auth_key"], name: "idx_on_endpoint_p256dh_key_auth_key_7553014576"
    t.index ["user_id"], name: "index_push_subscriptions_on_user_id"
  end

  create_table "rooms", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.bigint "creator_id", null: false
    t.datetime "deleted_at"
    t.datetime "destroy_enqueued_at"
    t.string "direct_member_key"
    t.string "icon_name"
    t.string "inbound_email_token"
    t.string "name"
    t.datetime "pins_changed_at"
    t.string "type", null: false
    t.datetime "updated_at", null: false
    t.index ["direct_member_key"], name: "index_rooms_on_direct_member_key", unique: true, where: "direct_member_key IS NOT NULL AND deleted_at IS NULL"
    t.index ["inbound_email_token"], name: "index_rooms_on_inbound_email_token", unique: true
  end

  create_table "saved_items", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.integer "message_id", null: false
    t.datetime "remind_at"
    t.datetime "reminded_at"
    t.string "status", default: "in_progress", null: false
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["message_id"], name: "index_saved_items_on_message_id"
    t.index ["remind_at"], name: "index_saved_items_on_remind_at", where: "remind_at IS NOT NULL AND reminded_at IS NULL"
    t.index ["user_id", "message_id"], name: "index_saved_items_on_user_id_and_message_id", unique: true
    t.index ["user_id", "status"], name: "index_saved_items_on_user_id_and_status"
    t.index ["user_id"], name: "index_saved_items_on_user_id"
  end

  create_table "scheduled_messages", force: :cascade do |t|
    t.datetime "claimed_at"
    t.datetime "created_at", null: false
    t.datetime "dropped_at"
    t.text "markdown_source", null: false
    t.integer "reply_to_message_id"
    t.integer "room_id", null: false
    t.datetime "send_at", null: false
    t.datetime "sent_at"
    t.integer "sent_message_id"
    t.integer "thread_id"
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["reply_to_message_id"], name: "index_scheduled_messages_on_reply_to_message_id"
    t.index ["room_id"], name: "index_scheduled_messages_on_room_id"
    t.index ["send_at", "sent_at", "dropped_at"], name: "index_scheduled_messages_on_send_at_and_sent_at_and_dropped_at"
    t.index ["sent_message_id"], name: "index_scheduled_messages_on_sent_message_id"
    t.index ["thread_id"], name: "index_scheduled_messages_on_thread_id"
    t.index ["user_id", "send_at"], name: "index_scheduled_messages_on_user_id_and_send_at"
    t.index ["user_id"], name: "index_scheduled_messages_on_user_id"
  end

  create_table "searches", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.string "dedup_key"
    t.string "query", null: false
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["user_id", "dedup_key"], name: "index_searches_on_user_and_dedup_key", unique: true
    t.index ["user_id"], name: "index_searches_on_user_id"
  end

  create_table "sessions", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.string "ip_address"
    t.datetime "last_active_at", null: false
    t.string "token", null: false
    t.datetime "updated_at", null: false
    t.string "user_agent"
    t.integer "user_id", null: false
    t.index ["token"], name: "index_sessions_on_token", unique: true
    t.index ["user_id"], name: "index_sessions_on_user_id"
  end

  create_table "streams", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.datetime "ended_at"
    t.integer "membership_id", null: false
    t.string "quality", null: false
    t.integer "room_id", null: false
    t.datetime "started_at", null: false
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["membership_id"], name: "index_streams_on_membership_id"
    t.index ["room_id"], name: "index_streams_on_room_id", unique: true, where: "ended_at IS NULL"
  end

  create_table "thread_memberships", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.string "involvement", default: "mentions", null: false
    t.datetime "joined_at", null: false
    t.integer "thread_id", null: false
    t.datetime "unread_at"
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["thread_id", "unread_at"], name: "index_thread_memberships_on_thread_id_and_unread_at"
    t.index ["thread_id", "user_id"], name: "index_thread_memberships_on_thread_id_and_user_id", unique: true
    t.index ["user_id"], name: "index_thread_memberships_on_user_id"
  end

  create_table "thread_tags", force: :cascade do |t|
    t.integer "channel_thread_id", null: false
    t.datetime "created_at", null: false
    t.string "name", null: false
    t.datetime "updated_at", null: false
    t.index ["channel_thread_id", "name"], name: "index_thread_tags_on_channel_thread_id_and_name", unique: true
    t.index ["name"], name: "index_thread_tags_on_name"
  end

  create_table "twitter_post_references", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.integer "message_id", null: false
    t.integer "twitter_post_id", null: false
    t.datetime "updated_at", null: false
    t.index ["message_id", "twitter_post_id"], name: "index_twitter_post_references_on_message_and_post", unique: true
    t.index ["message_id"], name: "index_twitter_post_references_on_message_id"
    t.index ["twitter_post_id"], name: "index_twitter_post_references_on_twitter_post_id"
  end

  create_table "twitter_posts", force: :cascade do |t|
    t.string "author_avatar_url"
    t.string "author_handle"
    t.string "author_name"
    t.datetime "created_at", null: false
    t.string "fetch_error"
    t.datetime "fetch_requested_at"
    t.datetime "fetched_at"
    t.integer "likes"
    t.json "media"
    t.string "post_id", null: false
    t.datetime "posted_at"
    t.json "quote"
    t.integer "replies"
    t.integer "reposts"
    t.text "text"
    t.datetime "updated_at", null: false
    t.string "url"
    t.index ["post_id"], name: "index_twitter_posts_on_post_id", unique: true
  end

  create_table "users", force: :cascade do |t|
    t.text "bio"
    t.string "bot_token"
    t.string "bot_token_digest"
    t.datetime "created_at", null: false
    t.string "custom_status_emoji"
    t.datetime "custom_status_expires_at"
    t.string "custom_status_text"
    t.boolean "dnd_enabled", default: false, null: false
    t.datetime "dnd_until"
    t.string "email_address"
    t.datetime "email_self_changed_at"
    t.string "github_login"
    t.boolean "google_email_link_allowed", default: false, null: false
    t.string "icon_name"
    t.json "inbox_preferences", default: {}
    t.string "name", null: false
    t.string "password_digest"
    t.string "presence_setting", default: "auto", null: false
    t.boolean "quiet_hours_enabled", default: false, null: false
    t.integer "quiet_hours_end_minute"
    t.integer "quiet_hours_start_minute"
    t.integer "role", default: 0, null: false
    t.integer "status", default: 0, null: false
    t.string "theme", default: "system", null: false
    t.string "time_zone"
    t.boolean "time_zone_explicit", default: false, null: false
    t.datetime "updated_at", null: false
    t.index "LOWER(github_login)", name: "index_users_on_lower_github_login", unique: true, where: "github_login IS NOT NULL"
    t.index ["bot_token"], name: "index_users_on_bot_token", unique: true
    t.index ["bot_token_digest"], name: "index_users_on_bot_token_digest", unique: true
    t.index ["email_address"], name: "index_users_on_email_address", unique: true
  end

  create_table "webhooks", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.string "signing_secret"
    t.datetime "updated_at", null: false
    t.string "url"
    t.integer "user_id", null: false
    t.index ["user_id"], name: "index_webhooks_on_user_id"
  end

  create_table "work_thread_events", force: :cascade do |t|
    t.integer "actor_id"
    t.integer "channel_thread_id", null: false
    t.datetime "created_at", null: false
    t.string "event_type", null: false
    t.integer "from_owner_id"
    t.string "from_owner_name"
    t.string "from_status"
    t.json "metadata"
    t.integer "to_owner_id"
    t.string "to_owner_name"
    t.string "to_status"
    t.datetime "updated_at", null: false
    t.index ["actor_id"], name: "index_work_thread_events_on_actor_id"
    t.index ["channel_thread_id", "created_at"], name: "index_work_thread_events_on_thread_and_created_at"
    t.index ["channel_thread_id"], name: "index_work_thread_events_on_channel_thread_id"
    t.index ["event_type", "created_at"], name: "index_work_thread_events_on_type_and_created_at"
  end

  create_table "work_thread_links", force: :cascade do |t|
    t.integer "channel_thread_id", null: false
    t.datetime "created_at", null: false
    t.integer "created_by_id", null: false
    t.integer "event_id"
    t.integer "github_pull_request_id"
    t.string "kind", null: false
    t.string "title"
    t.datetime "updated_at", null: false
    t.string "url"
    t.index ["channel_thread_id", "event_id"], name: "index_work_thread_links_on_thread_and_event", unique: true, where: "event_id IS NOT NULL"
    t.index ["channel_thread_id", "kind", "github_pull_request_id"], name: "index_work_thread_links_on_thread_kind_and_pr", unique: true, where: "github_pull_request_id IS NOT NULL"
    t.index ["channel_thread_id", "url"], name: "index_work_thread_links_on_thread_and_url", unique: true, where: "url IS NOT NULL"
    t.index ["channel_thread_id"], name: "index_work_thread_links_on_channel_thread_id"
    t.index ["created_by_id"], name: "index_work_thread_links_on_created_by_id"
    t.index ["event_id"], name: "index_work_thread_links_on_event_id"
    t.index ["github_pull_request_id"], name: "index_work_thread_links_on_github_pull_request_id"
  end

  create_table "workspace_icons", force: :cascade do |t|
    t.datetime "created_at", null: false
    t.integer "creator_id", null: false
    t.string "name", null: false
    t.string "title", null: false
    t.datetime "updated_at", null: false
    t.index ["creator_id"], name: "index_workspace_icons_on_creator_id"
    t.index ["name"], name: "index_workspace_icons_on_name", unique: true
  end

  create_table "workspace_presence_leases", force: :cascade do |t|
    t.string "connection_id", null: false
    t.datetime "created_at", null: false
    t.datetime "expires_at", null: false
    t.datetime "last_active_at"
    t.integer "session_id", null: false
    t.datetime "updated_at", null: false
    t.integer "user_id", null: false
    t.index ["connection_id"], name: "index_workspace_presence_leases_on_connection_id", unique: true
    t.index ["expires_at"], name: "index_workspace_presence_leases_on_expires_at"
    t.index ["session_id"], name: "index_workspace_presence_leases_on_session_id"
    t.index ["user_id"], name: "index_workspace_presence_leases_on_user_id"
  end

  add_foreign_key "active_storage_attachments", "active_storage_blobs", column: "blob_id"
  add_foreign_key "active_storage_variant_records", "active_storage_blobs", column: "blob_id"
  add_foreign_key "activity_items", "users", on_delete: :cascade
  add_foreign_key "agent_slash_commands", "agents"
  add_foreign_key "agent_slash_commands", "rooms"
  add_foreign_key "bans", "users"
  add_foreign_key "boosts", "messages"
  add_foreign_key "calendar_push_channels", "users"
  add_foreign_key "channel_threads", "messages", column: "parent_message_id", on_delete: :nullify
  add_foreign_key "channel_threads", "rooms"
  add_foreign_key "channel_threads", "users", column: "creator_id"
  add_foreign_key "channel_threads", "users", column: "work_owner_id", on_delete: :nullify
  add_foreign_key "dnd_allowed_users", "users"
  add_foreign_key "dnd_allowed_users", "users", column: "allowed_user_id"
  add_foreign_key "drive_attachments", "messages"
  add_foreign_key "event_calendar_entries", "events"
  add_foreign_key "event_calendar_entries", "users"
  add_foreign_key "event_references", "events"
  add_foreign_key "event_references", "messages"
  add_foreign_key "fizzy_card_caches", "fizzy_cards"
  add_foreign_key "fizzy_card_caches", "users"
  add_foreign_key "fizzy_card_references", "fizzy_cards"
  add_foreign_key "fizzy_card_references", "messages"
  add_foreign_key "fizzy_connected_accounts", "users"
  add_foreign_key "github_connected_accounts", "users"
  add_foreign_key "github_notifications", "github_repository_subscriptions", column: "subscription_id", on_delete: :cascade
  add_foreign_key "github_notifications", "messages", on_delete: :nullify
  add_foreign_key "github_pull_request_references", "github_pull_requests"
  add_foreign_key "github_pull_request_references", "messages"
  add_foreign_key "github_pull_request_threads", "channel_threads"
  add_foreign_key "github_pull_request_threads", "github_pull_requests"
  add_foreign_key "github_pull_request_threads", "rooms"
  add_foreign_key "github_repository_subscriptions", "rooms", on_delete: :cascade
  add_foreign_key "github_repository_subscriptions", "users", column: "created_by_id", on_delete: :nullify
  add_foreign_key "google_accounts", "users"
  add_foreign_key "google_identities", "users"
  add_foreign_key "keyword_alerts", "users"
  add_foreign_key "link_embed_references", "link_embeds"
  add_foreign_key "link_embed_references", "messages"
  add_foreign_key "message_pins", "messages"
  add_foreign_key "message_pins", "rooms"
  add_foreign_key "message_pins", "users", column: "pinner_id"
  add_foreign_key "messages", "channel_threads", column: "thread_id", on_delete: :cascade
  add_foreign_key "messages", "messages", column: "forwarded_from_message_id", on_delete: :nullify
  add_foreign_key "messages", "messages", column: "reply_to_message_id", on_delete: :nullify
  add_foreign_key "messages", "rooms"
  add_foreign_key "messages", "users", column: "creator_id"
  add_foreign_key "poll_options", "polls"
  add_foreign_key "poll_votes", "poll_options"
  add_foreign_key "poll_votes", "polls"
  add_foreign_key "poll_votes", "users"
  add_foreign_key "polls", "messages"
  add_foreign_key "push_subscriptions", "users"
  add_foreign_key "saved_items", "messages"
  add_foreign_key "saved_items", "users"
  add_foreign_key "scheduled_messages", "channel_threads", column: "thread_id"
  add_foreign_key "scheduled_messages", "messages", column: "reply_to_message_id"
  add_foreign_key "scheduled_messages", "messages", column: "sent_message_id"
  add_foreign_key "scheduled_messages", "rooms"
  add_foreign_key "scheduled_messages", "users"
  add_foreign_key "searches", "users"
  add_foreign_key "sessions", "users"
  add_foreign_key "thread_memberships", "channel_threads", column: "thread_id", on_delete: :cascade
  add_foreign_key "thread_memberships", "users", on_delete: :cascade
  add_foreign_key "twitter_post_references", "messages"
  add_foreign_key "twitter_post_references", "twitter_posts"
  add_foreign_key "webhooks", "users"
  add_foreign_key "work_thread_events", "channel_threads", on_delete: :cascade
  add_foreign_key "work_thread_events", "users", column: "actor_id", on_delete: :nullify
  add_foreign_key "work_thread_links", "channel_threads", on_delete: :cascade
  add_foreign_key "work_thread_links", "events", on_delete: :cascade
  add_foreign_key "work_thread_links", "github_pull_requests"
  add_foreign_key "work_thread_links", "users", column: "created_by_id"
  add_foreign_key "workspace_icons", "users", column: "creator_id"
  add_foreign_key "workspace_presence_leases", "sessions", on_delete: :cascade
  add_foreign_key "workspace_presence_leases", "users", on_delete: :cascade

  # Virtual tables defined in this database.
  # Note that virtual tables may not work with other database engines. Be careful if changing database.
  create_virtual_table "message_search_index", "fts5", ["body", "tokenize=porter"]
end
