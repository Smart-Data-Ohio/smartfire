# Tools-only producer mutations. Loaded only into fresh Rails test servers.
# No assertion, browser response, or fixture is replaced.
Rails.application.config.after_initialize do
  mode = ENV.fetch('WS8BM_WORK_PRODUCER')
  case mode
  when 'missing-history'
    WorkThreadEvent.singleton_class.prepend(Module.new do
      def create_for_change!(**kwargs)
        nil
      end
    end)
  when 'allow-reassignment'
    ChannelThread.prepend(Module.new do
      def work_assignment_manageable_by?(user)
        true
      end
    end)
  when 'missing-agent-events'
    ChannelThread.prepend(Module.new do
      def record_work_assignment_events!(from_owner:, to_owner:, actor:)
        []
      end
    end)
  when 'empty-owner-options'
    MessagePayloadHelper.prepend(Module.new do
      def work_owner_options(thread)
        []
      end
    end)
  when 'rewrite-history-client-id'
    ChannelThread.prepend(Module.new do
      def update_work!(**kwargs)
        result = super
        messages.where(markdown_source: 'Keep this history').update_all(client_message_id: 'corrupted-work-history')
        result
      end
    end)
  when 'extra-foreign-event'
    ChannelThread.prepend(Module.new do
      def update_work!(**kwargs)
        result = super
        foreign = ChannelThread.find_by!(name: 'Revoked work owner')
        WorkThreadEvent.create!(thread: foreign, actor: kwargs.fetch(:actor), event_type: 'work_update', from_status: 'planned', to_status: 'in_progress')
        result
      end
    end)
  else
    raise "Unknown review producer #{mode}"
  end
  warn "WS8bm real Rails producer mutation installed: #{mode}"
end
