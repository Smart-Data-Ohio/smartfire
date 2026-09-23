module AuditLogsHelper
  # One-line human summary of an audit row's change payload: [ before, after ]
  # pairs render as "before → after", scalars as-is. Falls back to compact
  # JSON for anything unexpected. All values are member-controlled; the view
  # escapes them.
  def audit_changes_summary(details)
    return "—" if details.blank?

    details.filter_map do |key, value|
      case value
      in [ before, after ]
        "#{key}: #{audit_change_value(before)} → #{audit_change_value(after)}"
      else
        "#{key}: #{audit_change_value(value)}"
      end
    end.join("; ").presence || "—"
  end

  private
    def audit_change_value(value)
      case value
      when nil then "∅"
      when String then value.truncate(80)
      when Array, Hash then value.to_json.truncate(80)
      else value.to_s
      end
    end
end
