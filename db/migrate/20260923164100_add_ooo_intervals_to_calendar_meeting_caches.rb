class AddOooIntervalsToCalendarMeetingCaches < ActiveRecord::Migration[8.2]
  def change
    add_column :calendar_meeting_caches, :ooo_intervals, :json, null: false, default: []
  end
end
