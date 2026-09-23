module Google
  # Connects a member's Google account for one-way event publishing.
  # Connecting is the opt-in; disconnecting removes the connection and
  # every calendar entry the app created for the user.
  class ConnectionsController < ApplicationController
    before_action :ensure_configured

    def connect
      raw_state = SecureRandom.hex(16)
      session[:google_oauth_state] = raw_state
      redirect_to Google::Client.authorize_url(redirect_uri: google_callback_url, state: state_verifier.generate(raw_state),
          drive: drive_requested?),
        allow_other_host: true
    end

    def callback
      stored_state = session.delete(:google_oauth_state)
      verified_state = state_verifier.verified(params[:state].to_s)

      # A stale state (a second tab overwrote the session's) is a
      # recoverable UX dead end, not a client error: send the member
      # back to try again.
      unless valid_state?(verified_state, stored_state)
        return redirect_to user_profile_path, alert: "Google connection expired. Try again."
      end

      if params[:error].present?
        return redirect_to user_profile_path, alert: "Google Calendar connection was not approved."
      end

      account = Current.user.google_account || Current.user.build_google_account
      tokens = Google::Client.exchange_code(code: params[:code].to_s, redirect_uri: google_callback_url)
      account.assign_attributes(
        access_token: tokens["access_token"],
        access_token_expires_at: Time.current + tokens["expires_in"].to_i.seconds,
        disconnected_reason: nil
      )
      account.refresh_token = tokens["refresh_token"] if tokens["refresh_token"].present?
      account.scopes = tokens["scope"] if tokens["scope"].present?
      account.email = Google::Client.email_from_id_token(tokens["id_token"])
      account.save!

      # Google lets the member deselect scopes at consent: without
      # calendar.events nothing can publish, so say so instead of
      # claiming a connection the profile will not show.
      unless account.calendar?
        return redirect_to user_profile_path, alert: "Calendar permission was not granted. Reconnect to publish events."
      end

      enqueue_upcoming_syncs(Current.user)
      redirect_to user_profile_path, notice: "Google Calendar connected."
    rescue Google::Client::Error => error
      Rails.logger.warn "Google OAuth callback failed: #{error.class}"
      redirect_to user_profile_path, alert: "Could not connect Google Calendar. Try again."
    end

    def destroy
      if (account = Current.user.google_account)
        google_event_ids = Current.user.event_calendar_entries.pluck(:google_event_id)
        snapshot = account.cleanup_snapshot
        account_id = account.id
        Current.user.event_calendar_entries.delete_all
        # Meet links were minted through this connection: clear them so
        # event cards stop advertising links the app no longer manages.
        # The request flag stays set (and update_all fires no callbacks),
        # so reconnecting re-provisions each pending event.
        Event.where(organizer: Current.user).where.not(meet_link: [ nil, "" ]).update_all(meet_link: nil)
        if (channel = Calendar::PushChannel.find_by(user_id: Current.user.id))
          channel.stop_remote!
          channel.destroy!
        end
        account.destroy!
        Calendar::DisconnectCleanupJob.perform_later(google_event_ids, snapshot, account_id) if snapshot
      end

      redirect_to user_profile_path, notice: "Google Calendar disconnected."
    end

    private
      def ensure_configured
        head :not_found unless Google::Client.configured?
      end

      def drive_requested?
        params[:features].is_a?(Array) && params[:features].include?("drive")
      end

      def state_verifier
        Rails.application.message_verifier("google_oauth_state")
      end

      def valid_state?(verified_state, stored_state)
        verified_state.is_a?(String) && stored_state.is_a?(String) &&
          verified_state.bytesize == stored_state.bytesize &&
          Rack::Utils.secure_compare(verified_state, stored_state)
      end

      def enqueue_upcoming_syncs(user)
        EventAttendance.where(user:, response: Event::NOTIFYING_RESPONSES)
          .joins(:event).merge(Event.upcoming).pluck(:event_id).each do |event_id|
            Calendar::SyncEntryJob.perform_later(event_id, user.id)
          end
        # A fresh connection provisions Meet links that were requested
        # before the organizer connected Google.
        Event.where(organizer: user, meet_link_requested: true, meet_link: [ nil, "" ])
          .find_each { |event| Calendar::MeetLinkJob.perform_later(event.id) }
        Calendar::WatchChannelJob.perform_later(user.id)
      end
  end
end
