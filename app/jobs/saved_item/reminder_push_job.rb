class SavedItem::ReminderPushJob < ApplicationJob
  def perform(saved_item)
    SavedItem::ReminderPusher.new(saved_item:).push
  end
end
