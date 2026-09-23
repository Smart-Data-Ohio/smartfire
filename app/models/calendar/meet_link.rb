module Calendar
  # Provisions a Google Meet link for an event through the organizer's
  # connected Google account, using Calendar conferenceData on the
  # organizer's own calendar copy (the existing calendar.events scope).
  # No Meet link unless the organizer has connected Google: without a
  # usable account the request waits, and connecting later provisions
  # every still-pending event the member organizes.
  class MeetLink
    def self.provision!(event)
      new(event).provision!
    end

    def initialize(event)
      @event = event
    end

    def provision!
      return unless @event.meet_link_requested? && @event.meet_link.blank? && !@event.cancelled?

      account = @event.organizer.google_account
      return unless account&.usable? && account.calendar?

      # The conference is attached to the organizer's copy, so make sure
      # it exists first; a failed sync leaves nothing to attach to.
      Calendar::EntrySync.sync(@event.id, @event.organizer_id)
      entry = EventCalendarEntry.find_by(event: @event, user_id: @event.organizer_id)
      return if entry.nil? || entry.synced_at.nil? || entry.last_error.present?

      response = Google::Client.new(account).update_event(
        entry.google_event_id,
        { "conferenceData" => { "createRequest" => { "requestId" => "meet-#{@event.id}-#{entry.id}" } } },
        conference_data_version: true
      )
      link = response.is_a?(Hash) ? response["hangoutLink"].presence : nil
      @event.update!(meet_link: link) if link
    rescue Google::Client::Unavailable
      raise
    rescue Google::Client::Error => error
      Rails.logger.warn "Calendar::MeetLink failed for event #{@event.id}: #{error.class}"
    end
  end
end
