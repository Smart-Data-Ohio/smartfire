class Users::TimeZonesController < ApplicationController
  # The browser reports its own zone on first visit; a zone the member
  # chose by hand always wins over a later detection.
  def update
    zone = params[:time_zone].to_s

    if ActiveSupport::TimeZone[zone].present? && Current.user.time_zone.blank?
      Current.user.update!(time_zone: zone)
    end

    render json: { time_zone: Current.user.reload.time_zone }
  rescue ActiveRecord::RecordInvalid
    render json: { time_zone: Current.user.reload.time_zone }, status: :unprocessable_entity
  end
end
