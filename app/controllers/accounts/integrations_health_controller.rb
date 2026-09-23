class Accounts::IntegrationsHealthController < ApplicationController
  before_action :ensure_can_administer

  # Admin-only integration health: GitHub, Google, Fizzy, forward-to-room
  # email, and webhook/agent delivery state. Read-only rollups — counts,
  # recent errors, disconnected accounts, and push channel expiry — with
  # a setup hint wherever something is disabled while unconfigured. Never
  # renders tokens or secrets.
  def show
    @github = github_snapshot
    @google = google_snapshot
    @fizzy = Integrations::FizzyStatus.snapshot
    @agent_delivery = agent_delivery_snapshot
    @email = email_snapshot
  end

  private
    def github_snapshot
      {
        workspace_token: ENV["GITHUB_TOKEN"].present?,
        app_configured: Github::App.configured?,
        webhook_secret: ENV["GITHUB_WEBHOOK_SECRET"].present?,
        connected: GithubConnectedAccount.where(disconnected_reason: nil).count,
        app_tokens: GithubConnectedAccount.where(disconnected_reason: nil, token_source: "app").count,
        disconnected: GithubConnectedAccount.where.not(disconnected_reason: nil)
          .order(updated_at: :desc).limit(10).pluck(:github_login, :disconnected_reason),
        last_errors: GithubConnectedAccount.where.not(last_error: nil)
          .order(updated_at: :desc).limit(10).pluck(:github_login, :last_error),
        deliveries_24h: Github::WebhookDelivery.where("created_at >= ?", 24.hours.ago).count,
        fetch_errors: Github::PullRequest.where.not(fetch_error: nil).order(updated_at: :desc).limit(10)
          .pluck(:owner, :repo, :number, :fetch_error)
      }
    end

    def google_snapshot
      {
        configured: Google::Client.configured?,
        connected: GoogleAccount.where(disconnected_reason: nil).count,
        disconnected: GoogleAccount.where.not(disconnected_reason: nil)
          .order(updated_at: :desc).limit(10).pluck(:email, :disconnected_reason),
        entry_errors: EventCalendarEntry.where.not(last_error: nil)
          .order(updated_at: :desc).limit(10).pluck(:event_id, :user_id, :last_error),
        push: {
          enabled: Calendar::PushChannel.watching_enabled?,
          count: Calendar::PushChannel.count,
          expiring: Calendar::PushChannel.where("expires_at IS NULL OR expires_at <= ?", 24.hours.from_now)
            .order(:expires_at).limit(10).pluck(:user_id, :expires_at, :last_error)
        }
      }
    end

    def agent_delivery_snapshot
      {
        pending: AgentEvent.where(webhook_status: "pending").count,
        failed: AgentEvent.where(webhook_status: "failed").where("created_at >= ?", 24.hours.ago).count,
        recent_errors: AgentEvent.where.not(webhook_last_error: nil)
          .order(created_at: :desc).limit(10).pluck(:agent_id, :event_type, :webhook_last_error)
      }
    end

    def email_snapshot
      {
        enabled: Room.inbound_email_enabled?,
        rooms_with_addresses: Room.alive.where.not(inbound_email_token: nil).count
      }
    end
end
