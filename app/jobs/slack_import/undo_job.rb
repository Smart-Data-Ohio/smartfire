class SlackImport::UndoJob < ApplicationJob
  queue_as :slack_import

  def perform(import_id)
    run = SlackImport.find_by(id: import_id)
    return if run.nil? || !run.undoing?
    return unless run.acquire_step_lease!("undoing")

    begin
      outcome = SlackImport::Undoer.new(run).step!
      self.class.perform_later(run.id) if outcome == :continue
    ensure
      run.release_step_lease!
    end
    SlackImport.kick_next_queued! if outcome == :stopped
  end
end
