module AuditLogsHelper
  # One-line human summary of an audit row's change payload: values the
  # recorder marked with AuditLog.pair render as "before → after",
  # scalars as-is. Falls back to compact JSON for anything unexpected.
  # All values are member-controlled; the view escapes them.
  def audit_changes_summary(details)
    return "—" if details.blank?

    details.filter_map do |key, value|
      pair = audit_before_after(value)
      if pair
        "#{key}: #{audit_change_value(pair[0])} → #{audit_change_value(pair[1])}"
      else
        "#{key}: #{audit_change_value(value)}"
      end
    end.join("; ").presence || "—"
  end

  private
    # Explicitly marked pairs only: a plain two-element array (a pair of
    # names, a short list) is a list, not a before/after. Accepts string
    # or symbol keys since callers may pass unpersisted payloads.
    def audit_before_after(value)
      return nil unless value.is_a?(Hash) && value.size == 2

      if value.key?("before") && value.key?("after")
        [ value["before"], value["after"] ]
      elsif value.key?(:before) && value.key?(:after)
        [ value[:before], value[:after] ]
      end
    end

    def audit_change_value(value)
      case value
      when nil then "∅"
      when String then value.truncate(80)
      when Array, Hash then value.to_json.truncate(80)
      else value.to_s
      end
    end
end
