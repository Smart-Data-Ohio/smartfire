module SetTimeZone
  extend ActiveSupport::Concern

  included do
    # Registration order is execution order, and an around hook only
    # wraps what follows it, so the isolator registers first: it then
    # wraps the applier and the action, restoring the previous zone even
    # when the action raises, so a zone never leaks across requests
    # sharing a thread. The applier still runs after Authentication's
    # own hooks (see the separate include in ApplicationController), so
    # Current.user is set when it runs.
    around_action :isolate_time_zone
    before_action :apply_user_time_zone
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
