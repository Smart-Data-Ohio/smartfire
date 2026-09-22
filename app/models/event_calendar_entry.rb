# The Google Calendar copy of one event for one member. Each connected
# member gets their own private copy on their primary calendar.
class EventCalendarEntry < ApplicationRecord
  belongs_to :event
  belongs_to :user

  validates :google_event_id, presence: true
  validates :user_id, uniqueness: { scope: :event_id }

  # Room destroys and series rebuilds drop local rows without touching
  # Google: capture what the remote delete needs before destroy and
  # enqueue it after commit, so the Google copy is removed however the
  # destroy was triggered. Callers that already removed the remote copy
  # (the sync reconciler) or could never reach it (a rejected account,
  # disconnect's own cleanup job) use delete/delete_all to skip this.
  before_destroy :capture_remote_delete_snapshot
  after_destroy_commit :enqueue_remote_delete

  private
    def capture_remote_delete_snapshot
      @remote_delete_user_id = user_id
      @remote_delete_google_event_id = google_event_id
    end

    def enqueue_remote_delete
      Calendar::RemoteDeleteJob.perform_later(@remote_delete_user_id, @remote_delete_google_event_id)
    end
end
