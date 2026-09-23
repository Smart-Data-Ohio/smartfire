class Autocompletable::SlashCommandsController < ApplicationController
  # JSON source for the composer's "/" command picker: the built-in
  # commands available in this conversation plus the room's registered
  # agent commands. Names carry no leading slash so the collection
  # matcher (which sees the query after "/") ranks them; the renderer
  # adds it back for display.
  def index
    room = Current.user.rooms.find(params[:room_id])
    thread = room.channel_threads.find(params[:thread_id]) if params[:thread_id].present?

    commands = SlashCommands::Registry.available_for(Current.user, room, thread: thread).map do |command|
      { name: command.name, value: command.name, description: command.description, arg_hint: command.arg_hint, agent: nil }
    end
    commands += room.agent_slash_commands.includes(:agent).ordered.map do |registration|
      {
        name: registration.name, value: registration.name,
        description: registration.description.presence || "Custom command",
        arg_hint: "", agent: registration.agent.user.name
      }
    end

    if params[:query].present?
      query = params[:query].to_s.downcase
      commands = commands.select { |command| command[:name].include?(query) || command[:description].to_s.downcase.include?(query) }
    end

    render json: commands
  rescue ActiveRecord::RecordNotFound
    head :not_found
  end
end
