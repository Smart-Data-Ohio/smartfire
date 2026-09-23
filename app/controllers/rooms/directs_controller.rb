class Rooms::DirectsController < RoomsController
  before_action :set_room, only: %i[ edit update destroy leave add_members ]

  def new
    @room = Rooms::Direct.new
  end

  def create
    users = selected_users

    if users.size > Rooms::Direct::MAX_MEMBERS
      redirect_to new_rooms_direct_path, alert: "Group direct messages hold at most #{Rooms::Direct::MAX_MEMBERS} people."
      return
    end

    room = Rooms::Direct.find_or_create_for(users)

    broadcast_create_room(room) if room.previously_new_record?
    redirect_to start_huddle? ? room_url(room, huddle: "start") : room_url(room)
  end

  def edit
  end

  def update
    @room.rename(room_params[:name], renamed_by: Current.user)
    redirect_to edit_rooms_direct_path(@room), notice: "Group renamed."
  rescue Rooms::Direct::NotAGroup
    redirect_to edit_rooms_direct_path(@room), alert: "Only group direct messages can be renamed."
  rescue ActiveRecord::RecordInvalid
    render :edit, status: :unprocessable_entity
  end

  def add_members
    added = @room.add_members(User.active.where(id: params[:user_ids]), added_by: Current.user)

    if added.any?
      redirect_to edit_rooms_direct_path(@room), notice: "Added #{added.map(&:name).to_sentence} to the group."
    else
      redirect_to edit_rooms_direct_path(@room), alert: "Select at least one new member to add."
    end
  rescue Rooms::Direct::NotAGroup
    redirect_to edit_rooms_direct_path(@room), alert: "Only group direct messages can add members."
  rescue Rooms::Direct::OverCapacity
    redirect_to edit_rooms_direct_path(@room), alert: "Group direct messages hold at most #{Rooms::Direct::MAX_MEMBERS} people."
  end

  def leave
    if @room.leave(Current.user) == :destroyed
      enqueue_destroy
      broadcast_remove_room
    end

    redirect_to root_url
  end

  private
    def selected_users
      User.where(id: selected_users_ids.including(Current.user.id))
    end

    def selected_users_ids
      params.fetch(:user_ids, [])
    end

    def start_huddle?
      params[:start_huddle].present?
    end

    def broadcast_create_room(room)
      room.memberships.each do |membership|
        membership.broadcast_prepend_to membership.user, :rooms, target: :direct_rooms, partial: "users/sidebars/rooms/direct"
      end
    end

    # All users in a direct room can administer it. Only direct rooms, though: this
    # relaxation is why room_scope below has to keep every other type out of reach.
    def ensure_can_administer
      true
    end

    def room_scope
      Current.user.rooms.directs
    end
end
