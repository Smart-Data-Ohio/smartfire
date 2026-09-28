class SlackImport::Undoer
  # Rows removed per job execution. Message and thread destroys run their
  # full callback paths, so batches stay small enough to finish in seconds.
  BATCH_SIZE = 200

  # Reverse dependency order: leaves first, rooms and users once nothing
  # points at them, the run's mapping rows last.
  UNDO_STEPS = %w[ leaves messages threads memberships rooms users records ].freeze

  def initialize(run)
    @run = run
    @state = { "phase" => "undo", "undo_step" => "leaves", "undo_cursor" => 0 }
      .merge(run.state || {})
  end

  # Removes one batch. Returns :continue (re-enqueue), :done (run undone)
  # or :stopped (run left the undoing state elsewhere).
  def step!
    return :stopped unless @run.reload.undoing?

    case @state["undo_step"]
    when "leaves" then step_leaves
    when "messages" then step_messages
    when "threads" then step_threads
    when "memberships" then step_memberships
    when "rooms" then step_rooms
    when "users" then step_users
    else step_records
    end
  end

  private
    def step_leaves
      batch = current_batch(%w[ reaction pin thread_membership ])
      return advance_to("messages") if batch.empty?

      batch.group_by(&:slack_kind).each do |kind, records|
        ids = records.map(&:record_id)
        case kind
        when "reaction" then Boost.where(id: ids).delete_all
        when "pin" then MessagePin.where(id: ids).delete_all
        when "thread_membership" then ThreadMembership.where(id: ids).delete_all
        end
      end
      save_cursor(batch)
      :continue
    end

    def step_messages
      batch = current_batch(%w[ message ])
      return advance_to("threads") if batch.empty?

      ids = batch.map(&:record_id)
      # Pins die quietly first: the message destroy would otherwise unpin
      # through the broadcasting path.
      MessagePin.where(message_id: ids).delete_all
      Message.where(id: ids).find_each do |message|
        message.importing = true
        begin
          message.destroy!
        rescue StandardError => error
          @run.record_issue!("error", "message:#{message.id}",
            "Could not remove imported message #{message.id}: #{error.message}")
        end
      end
      save_cursor(batch)
      :continue
    end

    def step_threads
      batch = current_batch(%w[ thread ])
      return advance_to("memberships") if batch.empty?

      ChannelThread.where(id: batch.map(&:record_id)).find_each do |thread|
        thread.importing = true
        begin
          thread.destroy!
        rescue StandardError => error
          @run.record_issue!("error", "thread:#{thread.id}",
            "Could not remove imported thread #{thread.id}: #{error.message}")
        end
      end
      save_cursor(batch)
      :continue
    end

    def step_memberships
      batch = current_batch(%w[ membership ])
      return advance_to("rooms") if batch.empty?

      Membership.where(id: batch.map(&:record_id)).delete_all
      save_cursor(batch)
      :continue
    end

    def step_rooms
      batch = current_batch(%w[ conversation ])
      return advance_to("users") if batch.empty?

      batch.each do |record|
        undo_room(record) if record.created_record?
      end
      save_cursor(batch)
      :continue
    end

    # A room the run created goes only when no messages remain in it other
    # than ones this run created (already removed above); a room someone
    # posted in since stays behind with an issue. Merged rooms are never
    # touched.
    def undo_room(record)
      room = Room.find_by(id: record.record_id)
      return if room.nil? || room.deleted?

      mine = @run.records.where(slack_kind: "message").select(:record_id)
      if room.messages.where.not(id: mine).exists?
        @run.record_issue!("warning", "room:#{room.id}",
          "Room #{room.name || room.id} kept: it holds messages this import did not create")
        return
      end

      begin
        room.destroy!
      rescue StandardError => error
        @run.record_issue!("error", "room:#{room.id}",
          "Could not remove imported room #{room.id}: #{error.message}")
      end
    end

    def step_users
      batch = current_batch(%w[ user ])
      return advance_to("records") if batch.empty?

      batch.each do |record|
        undo_user(record) if record.created_record?
      end
      save_cursor(batch)
      :continue
    end

    # Placeholder users go only if they never signed in (no sessions, no
    # password, no Google identity link) and author no remaining messages.
    # Matched users are never touched.
    def undo_user(record)
      user = User.find_by(id: record.record_id)
      return if user.nil?
      return if user.sessions.exists? || user.password_digest.present? ||
        GoogleIdentity.exists?(user_id: user.id) || user.messages.exists?

      begin
        user.destroy!
      rescue StandardError => error
        @run.record_issue!("error", "user:#{user.id}",
          "Could not remove placeholder user #{user.id}: #{error.message}")
      end
    end

    def step_records
      @run.records.delete_all
      stats = @run.stats.merge("phase" => "done", "current" => nil,
        "issues_count" => @run.issues.count)
      @run.update!(state: { "phase" => "done" }, stats:, status: "undone",
        finished_at: Time.current, heartbeat_at: Time.current)
      :done
    end

    def current_batch(kinds)
      @run.records.where(slack_kind: kinds)
        .where("slack_import_records.id > ?", @state["undo_cursor"].to_i)
        .order(:id).limit(BATCH_SIZE).to_a
    end

    def save_cursor(batch)
      @state["undo_cursor"] = batch.last.id
      @run.update!(state: @state, heartbeat_at: Time.current)
    end

    def advance_to(step)
      @state["undo_step"] = step
      @state["undo_cursor"] = 0
      @run.update!(state: @state, heartbeat_at: Time.current)
      :continue
    end
end
