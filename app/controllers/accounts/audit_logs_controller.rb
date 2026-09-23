require "csv"

class Accounts::AuditLogsController < ApplicationController
  before_action :ensure_can_administer
  before_action :no_store_response!

  PAGE_SIZE = 50
  CSV_EXPORT_LIMIT = 5000
  TARGET_TYPES = %w[ User Account Room Agent AgentCredential AgentGrant AgentApproval WorkspaceIcon ].freeze

  def show
    @filters = audit_filters
    entries = filtered_entries.order(id: :desc)

    respond_to do |format|
      format.html do
        set_page_and_extract_portion_from entries, per_page: PAGE_SIZE
        @entries = @page.records
      end
      format.csv do
        send_data audit_csv(entries.limit(CSV_EXPORT_LIMIT)),
          filename: "audit-log-#{Time.current.strftime("%Y%m%d-%H%M%S")}.csv",
          type: "text/csv", disposition: "attachment"
      end
    end
  end

  private
    def audit_filters
      {
        actor: params[:actor].to_s.strip.presence,
        # Named audit_action: params[:action] is the controller action.
        audit_action: params[:audit_action].to_s.presence_in(AuditLog::ACTIONS),
        target_type: params[:target_type].to_s.presence_in(TARGET_TYPES),
        from: parse_date(params[:from]),
        to: parse_date(params[:to])
      }
    end

    def filtered_entries
      entries = AuditLog.all
      if @filters[:actor]
        entries = entries.where("actor_label LIKE ?", "%#{AuditLog.sanitize_sql_like(@filters[:actor])}%")
      end
      entries = entries.where(action: @filters[:audit_action]) if @filters[:audit_action]
      entries = entries.where(target_type: @filters[:target_type]) if @filters[:target_type]
      entries = entries.where("created_at >= ?", @filters[:from].beginning_of_day) if @filters[:from]
      entries = entries.where("created_at <= ?", @filters[:to].end_of_day) if @filters[:to]
      entries
    end

    def parse_date(value)
      Date.parse(value.to_s) if value.present?
    rescue Date::Error
      nil
    end

    def audit_csv(entries)
      CSV.generate do |csv|
        csv << %w[ time action actor target_type target changes ip_address user_agent ]
        entries.each do |entry|
          csv << [
            entry.created_at.iso8601,
            entry.action,
            safe_csv_cell(entry.actor_label),
            entry.target_type,
            safe_csv_cell(entry.target_label),
            safe_csv_cell(entry.details.to_json),
            safe_csv_cell(entry.ip_address),
            safe_csv_cell(entry.user_agent)
          ]
        end
      end
    end

    # Actor names, room names, and change payloads are member-controlled:
    # neutralize spreadsheet formula injection in the export.
    def safe_csv_cell(value)
      return value if value.nil?

      text = value.to_s
      text.match?(/\A[=+\-@\t\r]/) ? "'#{text}" : text
    end
end
