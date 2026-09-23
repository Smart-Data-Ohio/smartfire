class AddRefreshPendingAtToCalendarMeetingCaches < ActiveRecord::Migration[8.2]
  def change
    add_column :calendar_meeting_caches, :refresh_pending_at, :datetime
  end
end
