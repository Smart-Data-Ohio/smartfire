# Tools-only injected clock. The unchanged Rails application uses Time.current.
# The fixed clock includes the browser's second visit, as travel_to's block does.
if ENV['LEDGER_BROWSER_CLOCK'] == '1'
  class << Time
    alias_method :ledger_original_now, :now
    def now
      path = '/rails/storage/db/ledger-clock.txt'
      File.exist?(path) ? Time.at(Integer(File.read(path).strip)).utc : ledger_original_now
    end
  end

  # The original system helper's perform_enqueued_jobs drains the UI-enqueued
  # Calendar job. Keep that queue in the actual rendering process, so neither
  # a Resque worker nor a separate rails runner can substitute a fresh job.
  Rails.application.config.after_initialize do
    Calendar::MeetingRefreshJob.queue_adapter = :test
    require 'webmock'
    include WebMock::API
    WebMock.enable!
    WebMock.disable_net_connect!(allow_localhost: true)
    stub_request(:get, 'https://www.googleapis.com/calendar/v3/calendars/primary/events')
      .with(query: hash_including({'singleEvents'=>'true','fields'=>Google::Client::MEETING_STATUS_FIELDS}))
      .to_return(status: 200, headers: {'Content-Type'=>'application/json'}, body: {items:[
        {status:'confirmed',start:{dateTime:'2026-03-02T15:55:00Z'},end:{dateTime:'2026-03-02T16:05:00Z'}},
        {status:'confirmed',start:{dateTime:'2026-03-02T15:54:00Z'},end:{dateTime:'2026-03-02T15:56:00Z'},transparency:'transparent'}
      ]}.to_json)
  end

  class LedgerCalendarQueue
    def initialize(app)
      @app, @sequence, @mutex = app, 0, Mutex.new
    end

    def call(env)
      path = '/rails/storage/db/ledger-calendar-request.json'
      @mutex.synchronize do
        if File.exist?(path)
          request = JSON.parse(File.read(path))
          if request.fetch('sequence') > @sequence
            @sequence = request.fetch('sequence')
            begin
              user = User.find(request.fetch('user'))
              case request.fetch('action')
              when 'refresh'
                raise 'original opt-in was not persisted' unless user.meeting_status_enabled?
                adapter = Calendar::MeetingRefreshJob.queue_adapter
                jobs = adapter.enqueued_jobs.select { |j| j[:job] == Calendar::MeetingRefreshJob }
                raise 'original UI did not enqueue exactly one Calendar job' unless jobs.length == 1 && jobs.first[:args] == [user.id]
                job = jobs.first
                adapter.enqueued_jobs.delete(job)
                adapter.performed_jobs << job
                ActiveJob::Base.execute(job)
                Calendar::MeetingDispatcher.dispatch_due!
                result = {enabled:true, enqueued_user:user.id}
              when 'advance'
                File.write('/rails/storage/db/ledger-clock.txt','1772467560')
                Calendar::MeetingDispatcher.dispatch_due!
                result = {clock:Time.current.iso8601}
              else
                raise 'unknown Calendar fixture action'
              end
              answer = {sequence:@sequence,result:result}
            rescue Exception => e
              answer = {sequence:@sequence,error:"#{e.class}: #{e.message}"}
            end
            temporary = '/rails/storage/db/ledger-calendar-response.tmp'
            File.write(temporary,answer.to_json)
            File.rename(temporary,'/rails/storage/db/ledger-calendar-response.json')
          end
        end
      end
      @app.call(env)
    end
  end
  Rails.application.config.middleware.insert_before(0, LedgerCalendarQueue)
end
