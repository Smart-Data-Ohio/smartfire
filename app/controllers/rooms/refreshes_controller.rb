class Rooms::RefreshesController < ApplicationController
  include RoomScoped

  # A refresh fired by a reconnect after the member was removed is routine,
  # not an error worth raising; the room UI is torn down by the broadcast.
  rescue_from ActiveRecord::RecordNotFound do
    head :not_found
  end

  before_action :set_last_updated_at

  def show
    @new_messages = Message::MentionPreloader.preload_for(
      @room.root_messages.with_rendering_details.page_created_since(@last_updated_at))
    @updated_messages = Message::MentionPreloader.preload_for(
      @room.root_messages.without(@new_messages).with_rendering_details.page_updated_since(@last_updated_at))
    # Pins touch their message (badges arrive through @updated_messages),
    # but the header count and panel list render only when the stamp moved.
    @pins_changed = @room.pins_changed_at.present? && @room.pins_changed_at > @last_updated_at
  end

  private
    def set_last_updated_at
      @last_updated_at = Time.at(0, params[:since].to_i, :millisecond)
    end
end
