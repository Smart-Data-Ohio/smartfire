class Rooms::PinsController < ApplicationController
  include RoomScoped

  rescue_from ActiveRecord::RecordNotFound, with: -> { head :not_found }

  def index
    @pins = @room.message_pins.ordered.includes(message: [ :room, :rich_text_body, { creator: :avatar_attachment } ])
  end
end
