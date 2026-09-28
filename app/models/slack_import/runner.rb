class SlackImport::Runner
  # Wall-clock bound per job execution: an execution stops starting new
  # units past this and re-enqueues, so a step never outruns Resque's
  # patience and progress saves incrementally.
  STEP_BUDGET = 25.seconds
  # Catch-up reads history from this far before the newest already-imported
  # message, picking up late thread replies without re-reading everything.
  CATCHUP_LOOKBACK = 30.days

  def initialize(run, client: nil)
    @run = run
    @workspace = run.slack_workspace
    @user_mapper = Slack::UserMapper.new(workspace: @workspace, run:)
    @conversation_mapper = Slack::ConversationMapper.new(workspace: @workspace, run:)
    @writer = Slack::MessageWriter.new(workspace: @workspace, run:, user_mapper: @user_mapper)
    @api_calls = run.stats["api_calls"] || 0
    @client = client || Slack::Client.new(token: run.slack_connection.access_token,
      on_request: ->(_method) { @api_calls += 1 })
    @state = with_state_defaults(run.state.deep_dup)
    @stats = with_stats_defaults(run.stats.deep_dup)
    @users_cache = {}
    @conversations_by_id = nil
    @stats_entries_by_id = nil
  end

  # Does a bounded unit of work. Returns :continue (re-enqueue), :done
  # (run completed) or :stopped (run cancelled or finished elsewhere).
  def step!
    @deadline = Time.current + STEP_BUDGET
    @state_before = @state.deep_dup
    check_running!

    outcome = catch(:stopped) do
      case @state["phase"]
      when "users" then step_users
      when "conversations" then step_conversations
      when "messages" then step_messages
      when "finishing" then step_finishing
      else step_finishing
      end
    end
    outcome == :stopped ? :stopped : outcome
  rescue ActiveRecord::RecordNotUnique
    # Two executions overlapped (a sweeper re-enqueue racing a slow step):
    # the loser rolls back and continues from the winner's saved state. If
    # nobody else saved, the conflict is real and must surface.
    raise if @run.reload.state == @state_before

    :continue
  end

  private
    def dry_run?
      @run.dry_run?
    end

    def import_mode?
      !dry_run?
    end

    def over_budget?
      Time.current >= @deadline
    end

    def check_running!
      throw(:stopped, :stopped) unless @run.reload.running?
    end

    # -- users ----------------------------------------------------------

    def step_users
      return transition_to("conversations") if @state["users_done"]

      page = @client.users_list(cursor: @state["users_cursor"].presence)
      delta = if dry_run?
        @user_mapper.preview_page(page["members"])
      else
        @user_mapper.map_page(page["members"])
      end
      merge_user_stats(delta)

      if (cursor = next_cursor(page))
        @state["users_cursor"] = cursor
      else
        @state["users_done"] = true
        @state["users_cursor"] = nil
      end
      save_progress!
      :continue
    end

    def merge_user_stats(delta)
      delta.each { |key, value| @stats["users"][key] += value }
    end

    # -- conversations --------------------------------------------------

    def step_conversations
      page = @client.conversations_list(types: list_types,
        cursor: @state["conversations_cursor"].presence)
      Array(page["channels"]).each do |channel|
        next unless in_scope?(channel)

        @state["conversations"] << conversation_entry(channel)
      end

      if (cursor = next_cursor(page))
        @state["conversations_cursor"] = cursor
        save_progress!
        return :continue
      end

      finish_conversation_discovery
      transition_to("messages")
    end

    def list_types
      if @run.personal?
        "im,mpim,private_channel"
      elsif @run.options["include_private"] == false
        "public_channel"
      else
        "public_channel,private_channel"
      end
    end

    def in_scope?(channel)
      if (only = @run.options["conversation_ids"]).present?
        only.include?(channel["id"])
      else
        true
      end
    end

    def conversation_entry(channel)
      { "id" => channel["id"], "name" => channel["name"].presence || channel["id"],
        "type" => Slack::ConversationMapper.conversation_type(channel),
        "archived" => !!channel["is_archived"],
        "num_members" => channel["num_members"] }
    end

    def finish_conversation_discovery
      @state["conversations"].sort_by! { |entry| entry["id"].to_s }
      @state["conversation_ids"] = @state["conversations"].map { |entry| entry["id"] }
      @state["convo_index"] = 0

      @stats["conversations"] = @state["conversations"].map do |entry|
        { "id" => entry["id"], "name" => entry["name"], "type" => entry["type"],
          "archived" => entry["archived"], "members" => entry["num_members"] || 0,
          "messages" => 0, "threads" => 0,
          "target" => { "action" => "pending", "room_id" => nil, "room_name" => entry["name"] },
          "done" => false }
      end

      Array(@run.options["conversation_ids"]).each do |id|
        next if @state["conversation_ids"].include?(id)

        @run.record_issue!("warning", "channel:#{id}",
          "Requested conversation #{id} was not found in this run's scope; skipped")
      end
    end

    # -- messages -------------------------------------------------------

    def step_messages
      loop do
        ids = @state["conversation_ids"] || []
        return transition_to("finishing") if @state["convo_index"].to_i >= ids.size

        process_conversation(ids[@state["convo_index"].to_i])
        check_running!
        break if over_budget?
      end
      save_progress!
      :continue
    end

    def process_conversation(id)
      convo = current_convo(id)
      entry = stats_entry(id)
      @stats["current"] = entry["name"]

      step_convo_members(convo, entry)
      return unless convo["members_done"]

      resolve_convo_target(convo, entry)
      return if convo["skipped"]

      step_convo_history(convo, entry)
      return unless convo_history_complete?(convo)

      advance_convo(entry)
    end

    def current_convo(id)
      convo = @state["convo"]
      if convo.is_a?(Hash) && convo["id"] == id
        convo
      else
        @state["convo"] = { "id" => id, "member_ids" => [], "members_cursor" => nil,
          "members_done" => false, "resolved" => false, "room_id" => nil,
          "skipped" => false, "direct" => false, "history_cursor" => nil,
          "history_done" => false, "thread_queue" => [], "thread_ts" => nil,
          "thread_message_id" => nil, "thread_cursor" => nil, "thread_state" => {} }
      end
    end

    def step_convo_members(convo, entry)
      return if convo["members_done"]

      loop do
        page = @client.conversations_members(channel: convo["id"],
          cursor: convo["members_cursor"].presence)
        convo["member_ids"] |= Array(page["members"])
        convo["members_cursor"] = next_cursor(page)
        if convo["members_cursor"].nil?
          convo["members_done"] = true
          entry["members"] = convo["member_ids"].size
          save_progress!
          break
        end
        save_progress!
        break if over_budget?
      end
    end

    def resolve_convo_target(convo, entry)
      return if convo["resolved"]

      conversation = conversations_by_id[convo["id"]] || { "id" => convo["id"] }
      if import_mode?
        users = convo_users(convo)
        result = @conversation_mapper.resolve(conversation,
          member_ids: convo["member_ids"], users:)
        if result.action == "create"
          @stats["counts"]["rooms_created"] += 1
        elsif result.action == "merge"
          @stats["counts"]["rooms_merged"] += 1
        end
        convo["room_id"] = result.room&.id
        convo["direct"] = !!result.room&.direct?
      else
        users = Slack::ConversationMapper.dry_users_for(convo["member_ids"])
        result = @conversation_mapper.preview(conversation,
          member_ids: convo["member_ids"], users:)
      end

      entry["target"] = { "action" => result.action, "room_id" => result.room&.id,
        "room_name" => target_name(result, conversation, convo) }
      convo["resolved"] = true
      convo["skipped"] = result.action == "skip"

      if convo["skipped"]
        advance_convo(entry)
      else
        save_progress!
      end
    end

    def target_name(result, conversation, convo)
      if result.room
        result.room.name.presence ||
          @conversation_mapper.describe_target(conversation, member_count: convo["member_ids"].size)
      else
        @conversation_mapper.describe_target(conversation, member_count: convo["member_ids"].size)
      end
    end

    def step_convo_history(convo, entry)
      room = import_mode? ? Room.alive.find(convo["room_id"]) : nil
      bounds = effective_bounds(convo["id"])

      loop do
        if thread_pending?(convo)
          step_thread(convo, entry, room, bounds)
        elsif !convo["history_done"]
          step_history_page(convo, entry, room, bounds)
        else
          break
        end
        save_progress!
        check_running!
        break if over_budget?
      end
    end

    def thread_pending?(convo)
      convo["thread_ts"].present? || convo["thread_queue"].any?
    end

    def convo_history_complete?(convo)
      convo["history_done"] && !thread_pending?(convo)
    end

    def step_history_page(convo, entry, room, bounds)
      page = @client.conversations_history(channel: convo["id"],
        oldest: slack_ts(bounds[:oldest]), latest: slack_ts(bounds[:latest]),
        cursor: convo["history_cursor"].presence)
      messages = Array(page["messages"])

      if import_mode?
        result = @writer.write_history_page(room:, conversation_id: convo["id"],
          messages:, bounds:, users: convo_users(convo))
        add_counts(result[:counts])
        convo["thread_queue"] |= result[:thread_parents]
        entry["messages"] += result[:counts]["messages"]
        entry["threads"] += result[:thread_parents].size
      else
        dry_result = @writer.dry_history_page(conversation_name: entry["name"],
          messages:, bounds:, samples_remaining: samples_remaining)
        add_counts(dry_result[:counts])
        @stats["samples"] += dry_result[:samples]
        entry["messages"] += dry_result[:counts]["messages"]
        entry["threads"] += dry_result[:counts]["threads"]
      end

      convo["history_cursor"] = next_cursor(page)
      convo["history_done"] = convo["history_cursor"].nil?
    end

    # One replies page of the current thread, or the next queued thread.
    # Dry runs never reach here: they count threads from reply_count.
    def step_thread(convo, entry, room, bounds)
      id = convo["id"]
      if convo["thread_ts"].nil?
        parent = convo["thread_queue"].shift
        convo["thread_ts"] = parent["ts"]
        convo["thread_message_id"] = parent["message_id"]
        convo["thread_cursor"] = nil
        convo["thread_state"] = {}
      end

      page = @client.conversations_replies(channel: id, ts: convo["thread_ts"],
        oldest: slack_ts(bounds[:oldest]), latest: slack_ts(bounds[:latest]),
        cursor: convo["thread_cursor"].presence)
      counts = @writer.write_replies_page(room:, conversation_id: id,
        parent_ts: convo["thread_ts"], parent_message_id: convo["thread_message_id"],
        messages: Array(page["messages"]), bounds:, users: convo_users(convo),
        direct: convo["direct"], thread_state: convo["thread_state"])
      add_counts(counts)

      convo["thread_cursor"] = next_cursor(page)
      if convo["thread_cursor"].nil?
        @writer.finish_thread(id, convo["thread_ts"])
        convo["thread_ts"] = nil
        convo["thread_message_id"] = nil
        convo["thread_cursor"] = nil
        convo["thread_state"] = {}
      end
    end

    def advance_convo(entry)
      entry["done"] = true
      @state["convo_index"] = @state["convo_index"].to_i + 1
      @state["convo"] = nil
      @stats["current"] = nil
      save_progress!
    end

    # -- bounds ---------------------------------------------------------

    def effective_bounds(conversation_id)
      oldest = time_bound(@run.options["oldest"])
      latest = time_bound(@run.options["latest"])
      if (catchup = catchup_oldest(conversation_id))
        oldest = [ oldest, catchup ].compact.max
      end
      { oldest:, latest: }
    end

    def time_bound(value)
      Time.iso8601(value.to_s).to_f if value.present?
    end

    def slack_ts(seconds)
      format("%.6f", seconds) unless seconds.nil?
    end

    # Conversations with earlier mapping fetch history only from 30 days
    # before the newest imported message, catching late thread replies.
    def catchup_oldest(conversation_id)
      prefix = "#{conversation_id}:"
      newest = SlackImport::Record.where(slack_workspace_id: @workspace.id, slack_kind: "message")
        .where("slack_key LIKE ?", "#{prefix}%")
        .pick("MAX(CAST(SUBSTR(slack_key, #{prefix.length + 1}) AS REAL))")
      newest ? newest - CATCHUP_LOOKBACK.to_f : nil
    end

    # -- finishing ------------------------------------------------------

    def step_finishing
      finish_rooms if import_mode?

      @stats["phase"] = "done"
      @stats["current"] = nil
      @stats["issues_count"] = @run.issues.count
      @stats["api_calls"] = @api_calls
      @run.update!(state: @state.merge("phase" => "done"), stats: @stats,
        status: "completed", finished_at: Time.current, heartbeat_at: Time.current)
      :done
    end

    # Imported DMs must not jump to the top of the sidebar: rooms the
    # import created take their last imported message time, rooms that
    # already existed keep the later of their time and that. Memberships
    # the run created point at the last imported message with no unread.
    def finish_rooms
      my_messages = @run.records.where(slack_kind: "message").select(:record_id)
      my_memberships = @run.records.where(slack_kind: "membership").select(:record_id)

      @run.records.where(slack_kind: "conversation").find_each do |record|
        room = Room.alive.find_by(id: record.record_id)
        next if room.nil?

        last_time = Message.where(room_id: room.id, id: my_messages).maximum(:created_at)
        next if last_time.nil?

        updated_at = record.created_record ? last_time : [ room.updated_at, last_time ].max
        room.update_columns(updated_at:)

        last_id = Message.where(room_id: room.id, id: my_messages)
          .order(created_at: :desc, id: :desc).pick(:id)
        Membership.where(room_id: room.id, id: my_memberships)
          .update_all(last_read_message_id: last_id, unread_at: nil, updated_at: Time.current)
      end
    end

    # -- helpers --------------------------------------------------------

    def convo_users(convo)
      @users_cache[convo["id"]] ||= ensured_users(convo["member_ids"])
    end

    def ensured_users(member_ids)
      users = @user_mapper.users_for(member_ids)
      (Array(member_ids) - users.keys).each do |slack_id|
        users[slack_id] = @user_mapper.ensure_author(slack_id)
      end
      users
    end

    def conversations_by_id
      @conversations_by_id ||= @state["conversations"].index_by { |entry| entry["id"] }
    end

    def stats_entry(id)
      @stats_entries_by_id ||= @stats["conversations"].index_by { |entry| entry["id"] }
      @stats_entries_by_id[id] ||=
        { "id" => id, "name" => id, "type" => "public_channel", "archived" => false,
          "members" => 0, "messages" => 0, "threads" => 0,
          "target" => { "action" => "pending", "room_id" => nil, "room_name" => id },
          "done" => false }.tap { |entry| @stats["conversations"] << entry }
    end

    def samples_remaining
      20 - @stats["samples"].size
    end

    def next_cursor(page)
      page.dig("response_metadata", "next_cursor").presence
    end

    def add_counts(delta)
      delta.each { |key, value| @stats["counts"][key] += value }
    end

    def transition_to(phase)
      @state["phase"] = phase
      @stats["phase"] = phase
      save_progress!
      :continue
    end

    def save_progress!
      @stats["issues_count"] = @run.issues.count
      @stats["api_calls"] = @api_calls
      @run.update!(state: @state, stats: @stats, heartbeat_at: Time.current)
    end

    def with_state_defaults(state)
      { "phase" => "users", "users_cursor" => nil, "users_done" => false,
        "conversations_cursor" => nil, "conversations" => [],
        "conversation_ids" => nil, "convo_index" => 0, "convo" => nil }
        .merge(state || {})
    end

    def with_stats_defaults(stats)
      { "phase" => "users",
        "users" => { "matched" => 0, "placeholders" => 0, "deactivated" => 0, "bots" => 0, "total" => 0 },
        "conversations" => [],
        "counts" => { "rooms_created" => 0, "rooms_merged" => 0, "messages" => 0,
          "replies" => 0, "threads" => 0, "reactions" => 0, "pins" => 0,
          "files_linked" => 0, "skipped" => 0 },
        "current" => nil, "samples" => [], "issues_count" => 0, "api_calls" => 0 }
        .deep_merge(stats || {})
    end
end
