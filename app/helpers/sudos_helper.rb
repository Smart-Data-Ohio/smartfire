module SudosHelper
  # Hidden fields rebuilding the stashed sudo request for the replay
  # form. Keys were validated as scalar and secret-free at stash time;
  # names are re-checked here so a tampered session cannot smuggle an
  # unexpected structure into the resubmitted form. (The cookie session
  # is encrypted, so this is defense in depth, not the only guard.)
  def sudo_replay_fields(params, prefix = nil)
    return "".html_safe unless params.is_a?(Hash)

    safe_join(params.flat_map do |key, value|
      name = prefix ? "#{prefix}[#{key}]" : key.to_s
      next [] unless key.is_a?(String) && name.match?(/\A[\w.\-\[\]]+\z/)

      case value
      when Hash
        sudo_replay_fields(value, name)
      when Array
        value.filter_map do |entry|
          hidden_field_tag("#{name}[]", entry) if scalar_replay_value?(entry)
        end
      else
        scalar_replay_value?(value) ? hidden_field_tag(name, value) : []
      end
    end)
  end

  private
    def scalar_replay_value?(value)
      value.is_a?(String) || value.is_a?(Numeric) || value == true || value == false || value.nil?
    end
end
