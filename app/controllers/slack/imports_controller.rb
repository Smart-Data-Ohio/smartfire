module Slack
  # The member's personal import page: connect Slack, preview (a dry
  # run over the member's own DMs, group DMs and private channels) and
  # import from that preview. Members see and act only on their own
  # personal runs; workspace runs and other members' runs 404 here.
  # Administrators view every run from the admin runs pages instead.
  class ImportsController < ApplicationController
    before_action :set_run, only: %i[ show cancel undo ]

    # The personal page. Works only once an administrator has connected
    # the workspace (team_id names the Slack workspace being migrated).
    def index
      @workspace = SlackWorkspace.current
      @connection = Current.user.slack_connection
      @runs = Current.user.slack_imports.personal.newest_first.includes(:user)
    end

    # Starts a personal preview (dry run) or a personal import from a
    # completed preview. One active run per member; the engine queues
    # runs globally, so a start during another member's run waits its
    # turn instead of failing.
    def create
      workspace = SlackWorkspace.current
      unless workspace&.team_id.present?
        return redirect_to slack_imports_path, alert: "An administrator needs to set up Slack import first."
      end
      connection = Current.user.slack_connection
      unless connection&.connected?
        return redirect_to slack_imports_path, alert: "Connect your Slack account first."
      end
      if Current.user.slack_imports.active.exists?
        return redirect_to slack_imports_path, alert: "You already have an import running. Wait for it to finish."
      end

      if params[:mode].to_s == "import"
        start_personal_import(workspace, connection)
      else
        run = SlackImport.start!(workspace:, user: Current.user, connection:,
          kind: "personal", mode: "dry_run", options: {})
        AuditLog.record!(action: "slack.import.start", target: run,
          changes: { kind: "personal", mode: "dry_run" })
        redirect_to slack_import_path(run), notice: "Preview started."
      end
    end

    def show
      @conversations = Array(@run.stats["conversations"])
      @queued_behind = @run.queued? && SlackImport.active.where.not(id: @run.id).exists?
    end

    def cancel
      if @run.cancel!
        AuditLog.record!(action: "slack.import.cancel", target: @run)
        redirect_to slack_import_path(@run), notice: "Import cancelled."
      else
        redirect_to slack_import_path(@run), alert: "That run already finished."
      end
    end

    def undo
      if @run.undo!
        AuditLog.record!(action: "slack.import.undo", target: @run)
        redirect_to slack_import_path(@run), notice: "Undo started."
      else
        redirect_to slack_import_path(@run), alert: "That run cannot be undone."
      end
    end

    private
      # Own personal runs only: anything else 404s, whether a workspace
      # run, another member's run, or a missing id.
      def set_run
        @run = Current.user.slack_imports.personal.find_by(id: params[:id])
        head :not_found unless @run
      end

      def start_personal_import(workspace, connection)
        dry_run = Current.user.slack_imports.personal.dry_run.completed.find_by(id: params[:dry_run_id])
        unless dry_run
          return redirect_to slack_imports_path, alert: "Run a preview first."
        end

        conversation_ids = Array(params[:conversation_ids]).select(&:present?)
        if conversation_ids.empty?
          return redirect_to slack_import_path(dry_run), alert: "Check at least one conversation to import."
        end

        known_ids = Array(dry_run.stats["conversations"]).map { |entry| entry["id"].to_s }
        conversation_ids &= known_ids
        if conversation_ids.empty?
          return redirect_to slack_import_path(dry_run), alert: "Those conversations are not in the preview."
        end

        run = SlackImport.start!(workspace:, user: Current.user, connection:,
          kind: "personal", mode: "import",
          options: { "conversation_ids" => conversation_ids })
        AuditLog.record!(action: "slack.import.start", target: run,
          changes: { kind: "personal", mode: "import" })

        redirect_to slack_import_path(run), notice: "Import started."
      end
  end
end
