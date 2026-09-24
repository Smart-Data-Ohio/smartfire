module User::Role
  extend ActiveSupport::Concern

  included do
    enum :role, %i[ member administrator bot ]
  end

  def can_administer?(record = nil)
    administrator? || self == record&.creator || record&.new_record?
  end

  # Deleting a room for everyone: group DMs hold shared history, so only
  # administrators delete them (members leave instead); every other room
  # follows the creator-or-administrator rule. RoomsController#destroy
  # enforces this and the sidebar menu mirrors it for display only.
  def can_delete_room?(room)
    return administrator? if room.direct? && room.group_capable?

    can_administer?(room)
  end
end
