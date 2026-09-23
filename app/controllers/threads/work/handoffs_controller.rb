class Threads::Work::HandoffsController < ApplicationController
  before_action :set_thread
  before_action :ensure_room_member
  before_action :ensure_work_thread
  before_action :ensure_can_hand_off

  # GET /threads/:thread_id/work/handoff/new. The handoff form: the
  # receiving agent plus the context package (summary, links, open
  # questions). Anyone who can work the thread may hand it off.
  def new
    no_store_response!
    set_form_assigns
  end

  # POST /threads/:thread_id/work/handoff. Hands the thread to another
  # agent: ownership transfers, Work history records the handoff, the
  # receiver gets a work_handed_off event, and the audit log records
  # work.handoff. HTML redirects to the thread; JSON renders the work
  # payload with the handoff package.
  def create
    receiver = Agent.find_by(id: params[:receiver_agent_id])

    if (error = WorkHandoff.receiver_error(@thread, receiver))
      return handoff_invalid_response(error)
    end

    begin
      @handoff = @thread.hand_off!(
        sender: Current.user,
        receiver_agent: receiver,
        summary: params[:summary].to_s,
        links: handoff_list_param(:links),
        open_questions: handoff_list_param(:open_questions)
      )
    rescue ActiveRecord::RecordInvalid => error
      return handoff_invalid_response(error.record.errors.full_messages.to_sentence)
    end

    respond_to do |format|
      format.html { redirect_to room_thread_path(@room, @thread), notice: "Work handed off to #{receiver.user.name}." }
      format.json do
        render json: Agents::WorkPayload.for(@thread.reload).merge(handoff: @handoff.payload), status: :created
      end
    end
  end

  private
    def set_thread
      @thread = ChannelThread.find_by(id: params[:thread_id])
      head :not_found if @thread.nil?
    end

    def ensure_room_member
      @room = @thread.room
      head :not_found unless @thread.work_viewable_by?(Current.user)
    end

    def ensure_work_thread
      head :unprocessable_content unless @thread.work?
    end

    # The sender must be able to work the thread: a manager or the
    # current owner. Anything else is 403.
    def ensure_can_hand_off
      head :forbidden unless @thread.work_manageable_by?(Current.user)
    end

    # Agents that may receive this thread right now: active agent
    # members holding post_messages and manage_threads, other than the
    # current owner. The receiver rule is re-checked on create.
    def set_form_assigns
      member_agent_ids = @room.memberships.where(user_id: Agent.select(:user_id)).select(:user_id)
      @receivers = Agent.includes(:user, :owner).where(user_id: member_agent_ids).select do |agent|
        WorkHandoff.receiver_error(@thread, agent).nil?
      end.sort_by { |agent| agent.user.name.to_s.downcase }
    end

    # Forms post one URL or question per line; API callers may send an
    # array instead. The model caps and validates either shape.
    def handoff_list_param(key)
      value = params[key]
      return [] if value.blank?

      value.is_a?(Array) ? value : value.to_s.split(/[\r\n]+/)
    end

    def handoff_invalid_response(error)
      @error = error

      respond_to do |format|
        format.html do
          set_form_assigns
          render :new, status: :unprocessable_entity
        end
        format.json { render json: { error: @error }, status: :unprocessable_entity }
      end
    end
end
