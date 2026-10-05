# Tools-only copies of the original system-test model producers. The renderer
# and the optional pre-browser rails runner share these actions; nothing is a
# public controller action or a replacement broadcast payload.
module LedgerOriginalFixtures
  DIRECTORY = Rails.root.join('storage', 'db').freeze

  class << self
    def prepare!
      # ActiveJob::TestHelper supplies a TestAdapter to the original system
      # tests. Keep the queue in this process, alongside ActionCable's adapter.
      ActiveJob::Base.queue_adapter = :test
    end

    def perform(request)
      case request.fetch('action')
      when 'reset-fixtures'
        Icons.expire_custom_cache!
        Rails.cache.clear
        ActionController::Base.cache_store.clear
        ActionCable.server.pubsub.clear
        # Calendar has its own TestAdapter in the clock initializer. Clear
        # both that queue and inherited/other loaded test-job queues, as the
        # original per-declaration ActiveJob::TestHelper setup does.
        ([ActiveJob::Base] + ActiveJob::Base.descendants).map(&:queue_adapter).uniq.each do |adapter|
          next unless adapter.respond_to?(:enqueued_jobs) && adapter.respond_to?(:performed_jobs)

          adapter.enqueued_jobs.clear
          adapter.performed_jobs.clear
        end
        { reset: true }
      when 'setup'
        setup_user(request)
      when 'refresh', 'advance'
        raise ArgumentError, 'Calendar must drain the actual UI queue through its Calendar middleware'
      when 'message'
        create_message(request)
      when 'messages'
        { messages: request.fetch('messages').map { |item| create_message(item) } }
      when 'broadcast'
        message = Message.find(request.fetch('message'))
        message.broadcast_create
        { id: message.id }
      when 'huddle-configured'
        { configured: Huddle.configured? }
      when 'environment'
        values = request.fetch('values')
        raise ArgumentError, 'fixture environment must be a map' unless values.is_a?(Hash)
        values.each do |name, value|
          raise ArgumentError, 'fixture environment key is outside Huddle helper' unless Huddle::REQUIRED_ENVIRONMENT.include?(name)
          raise ArgumentError, 'fixture environment value must be a string or null' unless value.nil? || value.is_a?(String)
        end
        previous = values.keys.to_h { |name| [name, ENV[name]] }
        values.each { |name, value| value.nil? ? ENV.delete(name) : ENV[name] = value }
        { previous: previous, configured: Huddle.configured? }
      when 'group-huddle'
        # This is the original David fixture's real Direct-room and grant
        # producer. issue! runs after_issued!, including the recipient activity
        # items and invitation broadcasts in this renderer's cable adapter.
        david = User.find(127326141)
        room = Current.set(user: david) do
          Rooms::Direct.find_or_create_for(request.fetch('users').map { |id| User.find(id) })
        end
        HuddleGrant.issue!(session: Session.find(request.fetch('session')),
          membership: room.memberships.find_by!(user_id: david.id))
        { room: room.id }
      when 'forgery-off'
        File.write(DIRECTORY.join('ledger-forgery-off'), '1')
        { disabled: true }
      when 'forgery-on'
        path = DIRECTORY.join('ledger-forgery-off')
        File.delete(path) if File.exist?(path)
        { disabled: false }
      else
        raise ArgumentError, 'unknown original fixture action'
      end
    end

    private
      def create_message(request)
        message = Room.find(request.fetch('room')).root_messages.create!(
          creator: User.find(request.fetch('creator')),
          body: request.fetch('body'), client_message_id: request.fetch('key'))
        message.broadcast_create
        { id: message.id }
      end

      def setup_user(request)
        david = User.find(request.fetch('user'))
        if request.fetch('connect_calendar', false)
          GoogleAccount.create!(user: david, email: "#{david.name.parameterize}@gmail.test",
            refresh_token: "refresh-token-#{david.id}", access_token: "access-token-#{david.id}",
            access_token_expires_at: 1.hour.from_now)
        end
        attachment_body = ApplicationController.render partial: 'users/mention', locals: { user: david }
        mention = "<action-text-attachment sgid=\"#{david.attachable_sgid}\" content-type=\"application/vnd.campfire.mention\" content=\"#{attachment_body.gsub('"', '&quot;')}\"></action-text-attachment>"
        { ready: true, mention: mention }
      end
  end
end
