# Exercise Active Record's real callback runner without replacing any callbacks in the app.
require 'active_support/testing/time_helpers'
extend ActiveSupport::Testing::TimeHelpers
trace = []
klass = Class.new(ActiveRecord::Base) do
  self.table_name = 'channel_threads'
  after_commit do
    trace << "first:#{id}"
    raise 'PR192 injected Rails after-commit error'
  end
  after_commit { trace << "later:#{id}" }
end
exception = nil
travel_to Time.utc(2026,3,2,16) do
  begin
    klass.transaction do
      [1900700040,1900700041].each do |id|
        klass.create!(id:id,name:'Callback probe',room_id:486777696,creator_id:394959859,last_activity_at:Time.current,created_at:Time.current,updated_at:Time.current)
      end
    end
  rescue => e
    exception = {class:e.class.name,message:e.message,open_transactions:ActiveRecord::Base.connection.open_transactions}
  end
end
source = ActiveRecord::ConnectionAdapters::Transaction.instance_method(:commit_records).source_location
puts JSON.pretty_generate(exception:exception,trace:trace,committed_ids:klass.where(id:[1900700040,1900700041]).order(:id).pluck(:id),commit_records_source:source)
raise 'wrong callback failure behavior' unless trace == ['first:1900700040'] && klass.where(id:[1900700040,1900700041]).count == 2 && exception[:open_transactions] == 0
