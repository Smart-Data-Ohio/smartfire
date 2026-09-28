class Accounts::SlackImportRunsController < ApplicationController
  before_action :ensure_can_administer
  before_action :set_run, only: %i[ show status plan start_import catch_up cancel undo ]

  ISSUES_PER_PAGE = 50

  # Every run, workspace and personal, newest first.
  def index
    @runs = SlackImport.newest_first.includes(:user)
  end

  # Starts a workspace dry run: reads the scope from Slack and reports
  # what an import would do without writing anything.
  def create
    workspace = SlackWorkspace.current
    connection = Current.user.slack_connection

    unless workspace&.team_id.present?
      return redirect_to account_slack_import_path, alert: "Connect your Slack account first."
    end
    unless connection&.connected?
      return redirect_to account_slack_import_path, alert: "Connect your Slack account first."
    end
    if SlackImport.active.exists?
      return redirect_to account_slack_import_runs_path, alert: "Another import is already running. Wait for it to finish."
    end

    run = SlackImport.start!(workspace:, user: Current.user, connection:,
      kind: "workspace", mode: "dry_run",
      options: {
        "include_private" => params[:include_private] != "0",
        "oldest" => parse_oldest(params[:oldest]),
        "latest" => parse_latest(params[:latest])
      }.compact)
    AuditLog.record!(action: "slack.import.start", target: run,
      changes: { kind: "workspace", mode: "dry_run" })

    redirect_to account_slack_import_run_path(run), notice: "Dry run started."
  end

  def show
    issues = @run.issues.order(:id)
    set_page_and_extract_portion_from issues, per_page: ISSUES_PER_PAGE
    @issues = @page.records
    @queued_behind = queued_behind?
  end

  # The live status frame polled from the run page. A frame's src may
  # not reference its own page (Turbo rejects a self-referencing
  # source), so polling reads here instead of reloading the run page.
  def status
    @queued_behind = queued_behind?
  end

  # The dry run's plan: which conversations import, into which rooms.
  def plan
    unless @run.workspace? && @run.dry_run? && @run.completed?
      return redirect_to admin_run_path(@run), alert: "The plan is ready when the dry run completes."
    end

    @conversations = Array(@run.stats["conversations"])
    @samples = Array(@run.stats["samples"])
    @rooms = Room.alive.where(type: %w[ Rooms::Open Rooms::Closed ]).ordered
  end

  # Starts a real import from a completed dry run's plan. Two presets:
  # test (checked conversations, recent messages, undoable) and full
  # (checked conversations, everything).
  def start_import
    unless @run.workspace? && @run.dry_run? && @run.completed?
      return redirect_to admin_run_path(@run), alert: "Start from a completed dry run."
    end
    if (blocker = start_blocker)
      return redirect_to plan_account_slack_import_run_path(@run), alert: blocker
    end

    preset = params[:preset].to_s == "full" ? "full" : "test"
    conversation_ids = Array(params[:conversation_ids]).select(&:present?)
    if conversation_ids.empty?
      return redirect_to plan_account_slack_import_run_path(@run), alert: "Check at least one conversation to import."
    end

    known_ids = Array(@run.stats["conversations"]).map { |entry| entry["id"].to_s }
    conversation_ids &= known_ids
    if conversation_ids.empty?
      return redirect_to plan_account_slack_import_run_path(@run), alert: "Those conversations are not in the dry run."
    end

    options = {
      "conversation_ids" => conversation_ids,
      "include_private" => @run.options["include_private"] != false,
      "room_targets" => room_targets_from(params, conversation_ids)
    }
    if preset == "test"
      options["oldest"] = parse_oldest(params[:oldest]) || 14.days.ago.beginning_of_day.iso8601
      options["latest"] = parse_latest(params[:latest])
    end

    run = SlackImport.start!(workspace: @run.slack_workspace, user: Current.user,
      connection: Current.user.slack_connection,
      kind: "workspace", mode: "import", options: options.compact)
    AuditLog.record!(action: "slack.import.start", target: run,
      changes: { kind: "workspace", mode: "import", preset: })

    redirect_to account_slack_import_run_path(run), notice: "#{preset == "full" ? "Full" : "Test"} import started."
  end

  # Repeats a completed full import with the same conversations and
  # targets, picking up what changed in Slack since. Safe to repeat:
  # already-imported objects are skipped, never duplicated.
  def catch_up
    unless @run.workspace? && @run.import? && @run.completed? && @run.options["oldest"].blank?
      return redirect_to admin_run_path(@run), alert: "Catch-up starts from a completed full import."
    end
    if (blocker = start_blocker)
      return redirect_to admin_run_path(@run), alert: blocker
    end

    options = @run.options.slice("conversation_ids", "room_targets", "include_private")
    run = SlackImport.start!(workspace: @run.slack_workspace, user: Current.user,
      connection: Current.user.slack_connection,
      kind: "workspace", mode: "import", options:)
    AuditLog.record!(action: "slack.import.start", target: run,
      changes: { kind: "workspace", mode: "import", preset: "catch_up" })

    redirect_to account_slack_import_run_path(run), notice: "Catch-up import started."
  end

  def cancel
    if @run.cancel!
      AuditLog.record!(action: "slack.import.cancel", target: @run)
      redirect_to admin_run_path(@run), notice: "Import cancelled."
    else
      redirect_to admin_run_path(@run), alert: "That run already finished."
    end
  end

  def undo
    if @run.undo!
      AuditLog.record!(action: "slack.import.undo", target: @run)
      redirect_to admin_run_path(@run), notice: "Undo started."
    else
      redirect_to admin_run_path(@run), alert: "That run cannot be undone."
    end
  end

  private
    def set_run
      @run = SlackImport.find_by(id: params[:id])
      head :not_found unless @run
    end

    # Admins view every run here, workspace and personal alike.
    def admin_run_path(run)
      account_slack_import_run_path(run)
    end

    def queued_behind?
      @run.queued? && SlackImport.active.where.not(id: @run.id).exists?
    end

    # Starting needs the admin's own live connection and a quiet queue.
    def start_blocker
      if Current.user.slack_connection&.connected?
        ("Another import is already running. Wait for it to finish." if SlackImport.active.exists?)
      else
        "Connect your Slack account first."
      end
    end

    def room_targets_from(params, conversation_ids)
      nested = params[:room_targets]
      targets = nested.is_a?(ActionController::Parameters) ? nested.to_unsafe_h : {}
      room_ids = Room.alive.where(type: %w[ Rooms::Open Rooms::Closed ]).pluck(:id).to_set
      targets.slice(*conversation_ids).transform_values do |value|
        case value.to_s
        when "new", "skip" then value.to_s
        when /\A\d+\z/ then room_ids.include?(value.to_i) ? value.to_i : nil
        end
      end.compact
    end

    # The engine parses bounds with Time.iso8601, which rejects bare
    # dates, so each bound is a full timestamp: oldest opens its day,
    # latest closes its own (a bare latest would exclude its day).
    def parse_oldest(value)
      Date.iso8601(value.to_s).in_time_zone.beginning_of_day.iso8601 if value.present?
    rescue Date::Error
      nil
    end

    def parse_latest(value)
      Date.iso8601(value.to_s).in_time_zone.end_of_day.iso8601 if value.present?
    rescue Date::Error
      nil
    end
end
