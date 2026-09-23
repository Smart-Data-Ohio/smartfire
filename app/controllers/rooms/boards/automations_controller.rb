class Rooms::Boards::AutomationsController < RoomsController
  before_action :set_board, only: %i[ show create_tag_assignment destroy_tag_assignment update_sla_rules ]
  before_action :ensure_can_administer, only: %i[ show create_tag_assignment destroy_tag_assignment update_sla_rules ]

  # GET /rooms/boards/:board_id/automations. The board's automation
  # rules: tag auto-assignments plus per-status SLA timers. The board's
  # creator or an administrator only; other members get 403.
  def show
    set_show_assigns
  end

  # POST .../automations/tag_assignments. Adds a "tag assigns to member"
  # rule. The assignee must be an active member able to own posts.
  def create_tag_assignment
    @assignment = @room.board_tag_assignments.new(
      tag: params[:tag],
      assignee_id: params[:assignee_id],
      created_by: Current.user
    )

    if @assignment.save
      AuditLog.record!(action: "board.automation.change", target: @room,
        changes: { tag_rule: "created", tag: @assignment.tag, assignee: @assignment.assignee.name })
      redirect_to board_automations_path(@room), notice: "Auto-assign rule added."
    else
      @tag_error = @assignment.errors.full_messages.to_sentence
      set_show_assigns
      render :show, status: :unprocessable_entity
    end
  end

  # DELETE .../automations/tag_assignments/:id. Removes one rule.
  def destroy_tag_assignment
    @assignment = @room.board_tag_assignments.find_by(id: params[:id])
    unless @assignment
      redirect_to board_automations_path(@room), alert: "Rule not found."
      return
    end

    tag = @assignment.tag
    assignee_name = @assignment.assignee.name
    @assignment.destroy!
    AuditLog.record!(action: "board.automation.change", target: @room,
      changes: { tag_rule: "removed", tag: tag, assignee: assignee_name })

    redirect_to board_automations_path(@room), notice: "Auto-assign rule removed."
  end

  # PATCH .../automations/sla_rules. Upserts the per-status SLA timers:
  # each status maps to nudge/escalate minutes, and a status with both
  # blank loses its rule. All four statuses validate before any writes.
  def update_sla_rules
    submitted = sla_rule_params
    updates = ChannelThread::WORK_STATUSES.to_h do |status|
      fields = submitted[status] || {}
      [ status, {
        nudge: fields[:nudge_after_minutes].to_s.strip,
        escalate: fields[:escalate_after_minutes].to_s.strip
      } ]
    end

    errors = validate_sla_updates(updates)
    if errors.any?
      @sla_error = errors.to_sentence
      set_show_assigns
      return render :show, status: :unprocessable_entity
    end

    apply_sla_updates(updates)

    redirect_to board_automations_path(@room), notice: "SLA timers saved."
  end

  private
    def set_board
      @room = Current.user.rooms.boards.find_by(id: params[:board_id])

      redirect_to root_url, alert: "Room not found or inaccessible" unless @room
    end

    # Board rooms keep their type: only board rooms are in reach here.
    def room_scope
      Current.user.rooms.boards
    end

    def set_show_assigns
      @tag_assignments = @room.board_tag_assignments.includes(:assignee).order(:tag).to_a
      @sla_rules = @room.board_sla_rules.index_by(&:work_status)
      @candidates = @room.memberships.includes(:user).map(&:user).select(&:active?).sort_by { |user| user.name.to_s.downcase }
    end

    def sla_rule_params
      permitted = ChannelThread::WORK_STATUSES.to_h { |status| [ status, %i[ nudge_after_minutes escalate_after_minutes ] ] }
      params.permit(sla_rules: permitted).fetch(:sla_rules, {}).to_h
    end

    def validate_sla_updates(updates)
      updates.filter_map do |status, fields|
        next if fields[:nudge].blank? && fields[:escalate].blank?

        rule = @room.board_sla_rules.find_or_initialize_by(work_status: status)
        rule.nudge_after_minutes = fields[:nudge]
        rule.escalate_after_minutes = fields[:escalate]
        next if rule.valid?

        "#{ChannelThread::WORK_STATUS_LABELS.fetch(status)}: #{rule.errors.full_messages.to_sentence}"
      end
    end

    # Only changed rules write and audit: re-saving an unchanged form
    # writes nothing, like other audited settings.
    def apply_sla_updates(updates)
      updates.each do |status, fields|
        rule = @room.board_sla_rules.find_by(work_status: status)

        if fields[:nudge].blank? && fields[:escalate].blank?
          next unless rule

          rule.destroy!
          AuditLog.record!(action: "board.automation.change", target: @room,
            changes: { sla_rule: "removed", status: status })
          next
        end

        nudge = fields[:nudge].to_i
        escalate = fields[:escalate].to_i

        if rule.nil?
          @room.board_sla_rules.create!(work_status: status, nudge_after_minutes: nudge, escalate_after_minutes: escalate)
          AuditLog.record!(action: "board.automation.change", target: @room,
            changes: { sla_rule: "created", status: status, nudge_after_minutes: nudge, escalate_after_minutes: escalate })
        elsif rule.nudge_after_minutes != nudge || rule.escalate_after_minutes != escalate
          before = { nudge_after_minutes: rule.nudge_after_minutes, escalate_after_minutes: rule.escalate_after_minutes }
          rule.update!(nudge_after_minutes: nudge, escalate_after_minutes: escalate)
          AuditLog.record!(action: "board.automation.change", target: @room,
            changes: { sla_rule: "updated", status: status,
              nudge_after_minutes: AuditLog.pair(before[:nudge_after_minutes], nudge),
              escalate_after_minutes: AuditLog.pair(before[:escalate_after_minutes], escalate) })
        end
      end
    end
end
