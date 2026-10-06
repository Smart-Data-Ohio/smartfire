# Private file IPC for original fixture producers in the actual rendering
# process. ActionCable's test adapter is process-local, so a separate rails
# runner cannot supply the browser's message or huddle broadcasts.
if Rails.env.test?
  require 'json'
  require Rails.root.join('lib', 'ledger_fixture_actions').to_s

  Rails.application.config.after_initialize do
    LedgerOriginalFixtures.prepare!
  end

  class LedgerFixtureRpc
    def initialize(app)
      @app, @sequence, @mutex = app, 0, Mutex.new
      @request_path = LedgerOriginalFixtures::DIRECTORY.join('ledger-rpc-request.json')
      @response_path = LedgerOriginalFixtures::DIRECTORY.join('ledger-rpc-response.json')
    end

    def call(env)
      @mutex.synchronize { consume_request }
      @app.call(env)
    end

    private
      def consume_request
        return unless File.exist?(@request_path)

        request = JSON.parse(File.read(@request_path))
        sequence = request.fetch('sequence')
        raise ArgumentError, 'fixture sequence must be a positive integer' unless sequence.is_a?(Integer) && sequence.positive?
        return unless sequence > @sequence

        # A failing action also receives one acknowledgement and is never
        # executed again by a subsequent browser or health-check request.
        @sequence = sequence
        begin
          result = Rails.application.executor.wrap do
            ActiveRecord::Base.connection_pool.with_connection do
              LedgerOriginalFixtures.perform(request)
            end
          end
          answer = { sequence: sequence, result: result }
        rescue StandardError => error
          answer = { sequence: sequence, error: "#{error.class}: #{error.message}" }
        end
        temporary = "#{@response_path}.#{Process.pid}.tmp"
        File.write(temporary, JSON.generate(answer))
        File.rename(temporary, @response_path)
      end
  end

  Rails.application.config.middleware.insert_before(0, LedgerFixtureRpc)
end
