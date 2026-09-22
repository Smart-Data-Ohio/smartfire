module User::Bannable
  extend ActiveSupport::Concern

  # Agents the banned person owns stop too, as on deactivation: suspension
  # revokes their grants and refuses their credentials.
  def ban
    transaction do
      create_bans_from_sessions
      apply_ban
      Agent.where(owner_id: id).find_each(&:suspend!)
      banned!
    end
  end

  def unban
    transaction do
      bans.delete_all
      active!
    end
  end

  def remove_banned_content_later
    RemoveBannedContentJob.perform_later(self)
  end

  def remove_banned_content
    messages.each do |message|
      message.destroy
      message.broadcast_remove
    end
  end

  private
    def create_bans_from_sessions
      sessions.pluck(:ip_address).compact_blank.uniq.each do |ip|
        bans.create!(ip_address: ip)
      end
    end

    def apply_ban
      close_remote_connections
      sessions.delete_all
      remove_banned_content_later
    end
end
