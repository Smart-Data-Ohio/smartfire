class Rooms::SlashCommandsController < ApplicationController
  include RoomScoped

  rescue_from ActiveRecord::RecordNotFound, with: -> { head :not_found }

  before_action :ensure_active_human

  # POST /rooms/:room_id/slash_commands (JSON). Runs composer text
  # starting with "/" through the slash command dispatcher. Outcomes
  # are data (the request itself succeeded): posted (the message
  # broadcasts live), ephemeral/error (shown to the invoker only),
  # open_url (navigate, e.g. the prefilled event form), open_poll
  # (open the poll builder), or start_huddle.
  def create
    thread = @room.channel_threads.find(params[:thread_id]) if params[:thread_id].present?

    result = SlashCommands::Dispatcher.dispatch(
      user: Current.user, room: @room, thread: thread, text: params[:text].to_s
    )

    render json: slash_payload(result)
  end

  private
    def ensure_active_human
      head :forbidden unless Current.user&.active? && !Current.user.bot?
    end

    def slash_payload(result)
      case result.kind
      when :posted
        { status: "posted", message_id: result.payload[:message_id], notice: result.notice }.compact
      when :open_url
        { status: "open_url", url: result.url }
      when :open_poll
        { status: "open_poll" }
      when :start_huddle
        { status: "start_huddle", room_id: result.payload[:room_id], room_name: room_display_name(@room) }
      else
        { status: result.kind.to_s, message: result.message }
      end
    end
end
