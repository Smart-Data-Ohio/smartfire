module SetTimeZone
  extend ActiveSupport::Concern

  included do
    # Registered after Authentication's own hook (see the include order in
    # ApplicationController), so Current.user is set when this runs. The
    # around hook wraps the whole request and restores the previous zone
    # even when the action raises, so a zone never leaks across requests
    # sharing a thread.
    before_action :apply_user_time_zone
    around_action :isolate_time_zone
  end

  private
    def apply_user_time_zone
      zone = Current.user&.time_zone
      Time.zone = zone if zone.present? && ActiveSupport::TimeZone[zone]
    end

    def isolate_time_zone
      previous_zone = Time.zone
      yield
    ensure
      Time.zone = previous_zone
    end
end
