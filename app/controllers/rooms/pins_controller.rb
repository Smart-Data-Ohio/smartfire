class Rooms::PinsController < ApplicationController
  include RoomScoped

  def index
    @pins = @room.message_pins.ordered.includes(message: [ :room, :rich_text_body, { creator: :avatar_attachment } ])
  end
end
