class Accounts::SlackImportsController < ApplicationController
  before_action :ensure_can_administer
  before_action :require_sudo_mode, only: %i[ update destroy ]

  # The setup checklist: create the Slack app from the generated
  # manifest, save its credentials, connect Slack, then dry-run and
  # import. The Client Secret is write-only and never rendered back.
  def show
    @workspace = SlackWorkspace.current
    @manifest_json = Slack::AppManifest.to_json(base_url: request.base_url)
    @connection = Current.user.slack_connection
    @active_run = SlackImport.active.newest_first.first
  end

  def update
    @workspace = SlackWorkspace.current || SlackWorkspace.new
    @workspace.assign_attributes(
      client_id: params[:client_id].to_s.strip,
      configured_by: Current.user
    )
    # A blank secret field keeps the stored one; only a pasted value
    # replaces it. The secret never reaches the log or the audit row.
    @workspace.client_secret = params[:client_secret].to_s.strip if params[:client_secret].present?

    if @workspace.save
      AuditLog.record!(action: "slack.workspace.configure", target: @workspace,
        changes: { client_id: @workspace.client_id })
      redirect_to account_slack_import_path, notice: "Slack app credentials saved."
    else
      @manifest_json = Slack::AppManifest.to_json(base_url: request.base_url)
      @connection = Current.user.slack_connection
      @active_run = SlackImport.active.newest_first.first
      render :show, status: :unprocessable_content
    end
  end

  # Removes the app credentials and every member's connection. Run
  # history stays so past imports remain reviewable. Blocked while any
  # run is active.
  def destroy
    if SlackImport.active.exists?
      return redirect_to account_slack_import_path, alert: "Finish or cancel the running import first."
    end

    if (@workspace = SlackWorkspace.current)
      client_id = @workspace.client_id
      @workspace.connections.delete_all
      @workspace.update_columns(client_id: nil, client_secret: nil,
        team_id: nil, team_name: nil, team_domain: nil, configured_by_id: nil,
        updated_at: Time.current)
      AuditLog.record!(action: "slack.workspace.remove_credentials",
        changes: { client_id: })
    end

    redirect_to account_slack_import_path, notice: "Slack credentials removed."
  end
end
