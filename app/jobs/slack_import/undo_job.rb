class SlackImport::UndoJob < ApplicationJob
  queue_as :slack_import

  def perform(import_id)
    run = SlackImport.find_by(id: import_id)
    return if run.nil? || !run.undoing?

    outcome = SlackImport::Undoer.new(run).step!
    self.class.perform_later(run.id) if outcome == :continue
  end
end
