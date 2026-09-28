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
      "undo_rooms_decided" => false, "undo_messages_decided" => false,
      "undo_kept_room_ids" => [], "undo_kept_thread_ids" => [],
      "undo_kept_message_ids" => [] }
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
      decide_kept_messages!
      batch = current_batch(%w[ message ])
      return advance_to("threads") if batch.empty?

      # Parents of threads that stay behind stay too: the thread and its
      # surviving replies still point at them.
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

    # Messages carrying content the run did not create keep their row (and
    # their room, through the room check): polls, saved items and pins by
    # others all hang off the message, and the import creates none of them.
    # Decided once, before the first batch goes.
    def decide_kept_messages!
      return if @state["undo_messages_decided"]

      mine = @run.records.where(slack_kind: "message").select(:record_id)
      keep_message_content!(Poll.where(message_id: mine).pluck(:message_id), "a poll")
      keep_message_content!(SavedItem.where(message_id: mine).pluck(:message_id), "a saved item")
      keep_message_content!(MessagePin.where(message_id: mine).where.not(id: my_pin_ids).pluck(:message_id), "a pin")
      @state["undo_messages_decided"] = true
      save_undo_state!
    end

    def keep_message_content!(message_ids, what)
      Array(message_ids).each do |id|
        next if kept_message_ids.include?(id)

        (@state["undo_kept_message_ids"] ||= []) << id
        @run.record_issue!("warning", "message:#{id}",
          "Message #{id} kept: it holds #{what} this import did not create")
      end
    end

    # A thread parent in this batch whose thread must stay (it holds
    # foreign or kept messages) marks the thread and itself kept before
    # anything is removed: destroying the parent would leave the surviving
    # thread without one. A thread someone started on an imported message
    # after the import keeps that message the same way.
    def keep_live_thread_parents(message_ids)
      run_thread_parents.slice(*message_ids).each do |message_id, thread_id|
        next unless thread_must_stay?(thread_id)

        thread = ChannelThread.find_by(id: thread_id)
        keep_thread!(thread, message_id) if thread
      end

      foreign_parent_ids = ChannelThread.where(parent_message_id: message_ids)
        .where.not(id: my_thread_ids).pluck(:parent_message_id)
      keep_message_content!(foreign_parent_ids, "a thread")
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
        # landed mid-undo still saves the thread. Threads holding messages
        # undo kept (polls, saved items, pins) stay for the same reason.
        if kept_thread_ids.include?(thread.id) || thread_must_stay?(thread.id)
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
    # holding anything this run did not create keeps all its memberships
    # and its conversation mapping, with an issue recorded. Decided once;
    # the rooms step re-checks live before destroying.
    def decide_kept_rooms!
      return if @state["undo_rooms_decided"]

      seen = 0
      @run.records.where(slack_kind: "conversation", created_record: true).find_each do |record|
        @run.update_columns(heartbeat_at: Time.current) if (seen % 25).zero?
        seen += 1

        room = Room.find_by(id: record.record_id)
        next if room.nil? || room.deleted?
        next unless room_has_foreign_content?(room)

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

    def my_pin_ids
      @my_pin_ids ||= @run.records.where(slack_kind: "pin").select(:record_id)
    end

    def my_thread_ids
      @my_thread_ids ||= @run.records.where(slack_kind: "thread").select(:record_id)
    end

    def thread_must_stay?(thread_id)
      thread_has_foreign_messages?(thread_id) || thread_has_kept_messages?(thread_id)
    end

    def thread_has_foreign_messages?(thread_id)
      Message.where(thread_id: thread_id).where.not(id: my_message_ids).exists?
    end

    def thread_has_kept_messages?(thread_id)
      kept = kept_message_ids
      kept.any? && Message.where(thread_id: thread_id, id: kept).exists?
    end

    # Anything the run did not create keeps a room it sits in: foreign
    # messages, events, scheduled messages, pins and threads by others, and
    # every other content association Room destroys with itself (repository
    # subscriptions, board rows, agent slash commands). The import creates
    # only messages, threads, pins and memberships, so any row in the other
    # associations is foreign by definition. Polls and saved items hang off
    # messages and keep both the message and the room.
    def room_has_foreign_content?(room)
      room.messages.where.not(id: my_message_ids).exists? ||
        room.events.exists? ||
        room.scheduled_messages.exists? ||
        room.message_pins.where.not(id: my_pin_ids).exists? ||
        room.channel_threads.where.not(id: my_thread_ids).exists? ||
        room.github_repository_subscriptions.exists? ||
        room.board_tag_assignments.exists? || room.board_sla_rules.exists? ||
        room.board_sla_nudges.exists? || room.board_stale_digests.exists? ||
        room.agent_slash_commands.exists? ||
        Poll.where(message_id: room.messages.select(:id)).exists? ||
        SavedItem.where(message_id: room.messages.select(:id)).exists?
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
        "Room #{room.name || room.id} kept: it holds content this import did not create")
    end

    # A room the run created goes only when nothing remains in it that this
    # run did not create (its own messages were already removed above); a
    # room holding anything else stays behind with an issue. Merged rooms
    # are never touched.
    def undo_room(record)
      room = Room.find_by(id: record.record_id)
      return if room.nil? || room.deleted?
      return if kept_room_ids.include?(room.id)

      if room_has_foreign_content?(room)
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
      SlackImport.kick_next_queued!
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
      # Every placeholder user undo kept: without its mapping a re-import
      # cannot match it — deactivated and bot placeholders have no email —
      # and mints a duplicate. The mapping goes only with the user row.
      created_user_ids = @run.records.where(slack_kind: "user", created_record: true).pluck(:record_id)
      surviving_users = User.where(id: created_user_ids).pluck(:id)
      if surviving_users.any?
        ids |= @run.records.where(slack_kind: "user", record_id: surviving_users).pluck(:id)
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
