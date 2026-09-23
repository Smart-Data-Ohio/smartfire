# Serves the per-viewer quote frame for a cross-room message permalink:
# the quote card when the viewer belongs to the source room, a plain
# private-room chip otherwise. Same-room quotes render inline instead,
# because every viewer of the quoting message already belongs there.
# RoomScoped 404s non-members of the quoting room; the reference must
# belong to one of its messages.
class Rooms::MessageLinksController < ApplicationController
  include RoomScoped

  rescue_from ActiveRecord::RecordNotFound, with: -> { head :not_found }

  def show
    @reference = MessageReference.joins(:message)
      .where(messages: { room_id: @room.id }).find(params[:id])
    @source = @reference.referenced_message
    ActiveRecord::Associations::Preloader.new(
      records: [ @source ],
      associations: [ :room, :rich_text_body, { creator: :avatar_attachment } ]
    ).call

    @visible = helpers.message_quote_visible_to?(@source, Current.user)
    @frame_id = helpers.message_link_frame_id(@reference)

    render layout: false
  end
end
