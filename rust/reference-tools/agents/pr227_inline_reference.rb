# Producer control: preserve the real ledger/jobs and add one inline logical POST.
require "json"
require "net/http"
mode = ARGV.fetch(0)
raise "unknown inline control" unless %w[assignment_webhook assignment_no_read handoff_webhook].include?(mode)
inline = Module.new do
  define_method(:record_work_assignment_event!) do |agent, event_type, actor, hop, chain_id, extra_metadata: {}|
    result = super(agent, event_type, actor, hop, chain_id, extra_metadata: extra_metadata)
    selected = case mode
    when "assignment_webhook" then event_type == "work_assigned" && name == "Hooked work" && agent.id == 773018776
    when "assignment_no_read" then event_type == "work_assigned" && name == "Unread work" && agent.id == 773018776
    when "handoff_webhook" then event_type == "work_handed_off" && agent.id == 1901100002
    end
    if id == 1900700020 && selected
      configured = agent.user.webhook.url
      raise "unexpected logical webhook" unless configured == "http://93.184.216.34:8080/hook"
      uri = URI.parse(configured)
      request = Net::HTTP::Post.new(uri)
      request["Content-Type"] = "application/json"
      request.body = {event_id:result.id,event_type:result.event_type,agent_id:result.agent_id,thread_id:id,title:name}.to_json
      # The ordinary oracle's transport is already installed before this writer.
      Net::HTTP.start(uri.host, uri.port, ipaddr: uri.host) { |transport| transport.request(request) }
      warn "PR227_RAILS_INLINE_PRODUCER_HIT #{mode} #{configured}"
    end
    result
  end
  private :record_work_assignment_event!
end
ChannelThread.prepend(inline)
ARGV.replace(["pr227-delivery-inputs.json"])
load File.join(ENV.fetch("PARITY_WORK"), "reference-tools/agents/next6_named.rb")
