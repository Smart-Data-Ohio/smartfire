module SetTimeZone
  extend ActiveSupport::Concern

  included do
    around_action :use_user_time_zone
  end

  private
    def use_user_time_zone(&block)
      zone = Current.user&.time_zone

      if zone.present? && ActiveSupport::TimeZone[zone]
        Time.use_zone(zone, &block)
      else
        yield
      end
    end
end
