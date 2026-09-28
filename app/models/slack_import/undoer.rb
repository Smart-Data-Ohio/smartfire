class SlackImport::Undoer
  # Rows removed per job execution. Message and thread destroys run their
  # full callback paths, so batches stay small enough to finish in seconds.
  BATCH_SIZE = 200

  # Reverse dependency order: leaves first, rooms and users once nothing
  # points at them, the run's mapping rows last.
  UNDO_STEPS = %w[ leaves messages threads memberships rooms users records ].freeze

  def initialize(run)
    @run = run
    @state = { "phase" => "undo", "undo_step" => "leaves", "undo_cursor" => 0,
      "undo_rooms_decided" => false, "undo_kept_room_ids" => [],
      "undo_kept_thread_ids" => [], "undo_kept_message_ids" => [] }
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

      # Parents of threads holding foreign messages stay: the thread and
      # its surviving replies still point at them.
      keep_live_thread_parents(batch.map(&:record_id))
      ids = batch.map(&:record_id) - kept_message_ids
      # Pins die quietly first: the message destroy would otherwise unpin
      # through the broadcasting path. Only pins on messages actually
      # being removed are touched.
      MessagePin.where(message_id: ids).delete_all if ids.any?
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

    # A thread parent in this batch whose thread holds foreign messages
    # marks the thread (and itself) kept before anything is removed.
    def keep_live_thread_parents(message_ids)
      run_thread_parents.slice(*message_ids).each do |message_id, thread_id|
        next unless thread_has_foreign_messages?(thread_id)

        thread = ChannelThread.find_by(id: thread_id)
        keep_thread!(thread, message_id) if thread
      end
    end

    # This run's thread parents, as message id to thread id. Threads a
    # user deleted since the import are simply absent.
    def run_thread_parents
      @run_thread_parents ||= begin
        thread_ids = @run.records.where(slack_kind: "thread").pluck(:record_id)
        ChannelThread.where(id: thread_ids).pluck(:parent_message_id, :id).to_h
      end
    end

    def step_threads
      batch = current_batch(%w[ thread ])
      return advance_to("memberships") if batch.empty?

      ChannelThread.where(id: batch.map(&:record_id)).find_each do |thread|
        # A thread goes only when every message in it was created by this
        # run: destroy cascades into its replies, which would otherwise
        # kill real users' messages. The check runs live so a reply that
        # landed mid-undo still saves the thread.
        if kept_thread_ids.include?(thread.id) || thread_has_foreign_messages?(thread.id)
          keep_thread!(thread, thread.parent_message_id)
          next
        end

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
      decide_kept_rooms!
      batch = current_batch(%w[ membership ])
      return advance_to("rooms") if batch.empty?

      room_ids = Membership.where(id: batch.map(&:record_id)).pluck(:id, :room_id).to_h
      deletable = batch.map(&:record_id).reject { |id| kept_room_ids.include?(room_ids[id]) }
      Membership.where(id: deletable).delete_all if deletable.any?
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

    # Each room's fate is decided before any membership is touched: a room
    # holding messages this run did not create keeps all its memberships
    # and its conversation mapping, with an issue recorded. Decided once;
    # the rooms step re-checks live before destroying.
    def decide_kept_rooms!
      return if @state["undo_rooms_decided"]

      @run.records.where(slack_kind: "conversation", created_record: true).find_each do |record|
        room = Room.find_by(id: record.record_id)
        next if room.nil? || room.deleted?
        next unless room.messages.where.not(id: my_message_ids).exists?

        keep_room!(room)
      end
      @state["undo_rooms_decided"] = true
      save_undo_state!
    end

    # Messages this run created, as a subquery: anything outside this set
    # is foreign content that keeps its thread or room behind.
    def my_message_ids
      @my_message_ids ||= @run.records.where(slack_kind: "message").select(:record_id)
    end

    def thread_has_foreign_messages?(thread_id)
      Message.where(thread_id: thread_id).where.not(id: my_message_ids).exists?
    end

    def kept_room_ids
      Array(@state["undo_kept_room_ids"])
    end

    def kept_thread_ids
      Array(@state["undo_kept_thread_ids"])
    end

    def kept_message_ids
      Array(@state["undo_kept_message_ids"])
    end

    def keep_thread!(thread, parent_message_id)
      ids = (@state["undo_kept_thread_ids"] ||= [])
      return if ids.include?(thread.id)

      ids << thread.id
      @state["undo_kept_message_ids"] ||= []
      @state["undo_kept_message_ids"] |= [ parent_message_id ].compact
      @run.record_issue!("warning", "thread:#{thread.id}",
        "Thread #{thread.id} kept: it holds messages this import did not create")
      save_undo_state!
    end

    def keep_room!(room)
      ids = (@state["undo_kept_room_ids"] ||= [])
      return if ids.include?(room.id)

      ids << room.id
      @run.record_issue!("warning", "room:#{room.id}",
        "Room #{room.name || room.id} kept: it holds messages this import did not create")
    end

    # A room the run created goes only when no messages remain in it other
    # than ones this run created (already removed above); a room someone
    # posted in since stays behind with an issue. Merged rooms are never
    # touched.
    def undo_room(record)
      room = Room.find_by(id: record.record_id)
      return if room.nil? || room.deleted?
      return if kept_room_ids.include?(room.id)

      if room.messages.where.not(id: my_message_ids).exists?
        keep_room!(room)
        save_undo_state!
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
      # Mappings stay behind for everything undo deliberately kept, so a
      # later run reuses the surviving rooms, threads and messages instead
      # of duplicating them.
      keeper = kept_record_ids
      scope = keeper.any? ? @run.records.where.not(id: keeper) : @run.records
      scope.delete_all
      stats = @run.stats.merge("phase" => "done", "current" => nil,
        "issues_count" => @run.issues.count)
      @run.update!(state: { "phase" => "done" }, stats:, status: "undone",
        finished_at: Time.current, heartbeat_at: Time.current)
      :done
    end

    def kept_record_ids
      ids = []
      if kept_room_ids.any?
        ids |= @run.records.where(slack_kind: "conversation", record_id: kept_room_ids).pluck(:id)
        surviving = Membership.where(room_id: kept_room_ids).pluck(:id)
        ids |= @run.records.where(slack_kind: "membership", record_id: surviving).pluck(:id) if surviving.any?
      end
      if kept_thread_ids.any?
        ids |= @run.records.where(slack_kind: "thread", record_id: kept_thread_ids).pluck(:id)
      end
      if kept_message_ids.any?
        ids |= @run.records.where(slack_kind: "message", record_id: kept_message_ids).pluck(:id)
      end
      ids
    end

    def save_undo_state!
      @run.update!(state: @state, heartbeat_at: Time.current)
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
