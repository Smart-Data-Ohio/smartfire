# Maps one Slack object (a user, conversation, message, thread, reaction,
# pin or membership, identified by slack_kind + slack_key) to the Smartfire
# record it became. Unique per workspace, so every run skips what an
# earlier run (workspace or personal) already brought over. created_record
# is false when the Slack object was matched to a record that already
# existed (a member found by email, a merged room); undo never deletes those.
class SlackImport::Record < ApplicationRecord
  self.table_name = "slack_import_records"

  belongs_to :slack_workspace
  belongs_to :slack_import
  belongs_to :record, polymorphic: true, optional: true
end
