require "json"
Rails.logger = ActiveSupport::Logger.new($stderr)
Rails.logger.level = Logger::FATAL
class AgentProfileGoldenController < UsersController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
users = {david:127326141, kevin:712064548, jz:919958193}
# Resolve the parity seed's fixed JZ rather than assuming fixture-generated identifiers.
users[:jz] = User.find_by!(name: "JZ").id
bot = User.find(394959859)
agent = bot.agent || raise("no seeded agent")
original = agent.attributes
original_user = bot.attributes
room = Room.find(486777696)
original_room = room.attributes
inputs = [
  {name:"admin_grants", viewer:"david"},
  {name:"owner_grants", viewer:"kevin", attrs:{owner_id:users[:kevin]}},
  {name:"peer_hides_grants", viewer:"kevin"},
  {name:"identity_status_rooms_grants", viewer:"kevin", attrs:{provider:"OpenAI",runtime:"Codex CLI 0.9",description:"Does <& things",status:"working",status_note:"on <& it",status_changed_at:(Time.current-2.hours).iso8601}},
  {name:"owner_activity", viewer:"kevin", attrs:{owner_id:users[:kevin]}, activity:true},
  {name:"admin_activity", viewer:"david", attrs:{owner_id:users[:kevin]}},
  {name:"peer_hides_activity", viewer:"jz"},
  {name:"private_rooms_hidden", viewer:"jz"},
  {name:"suspended", viewer:"kevin", attrs:{suspended_at:Time.current.iso8601}},
  {name:"review_markup", viewer:"david", attrs:{provider:'<b>provider & "</b>',runtime:'<i>runtime</i>',description:'<script>description()</script>',status:"working",status_note:'<img src=x onerror="note()">'}, user_attrs:{name:'<b>Agent & "</b>',bio:'<i>agent bio</i>'}, room_attrs:{name:'<b>Room & "</b>'}},
  {name:"minimal_bot", viewer:"kevin", no_agent:true}
]
rows = inputs.map do |input|
  User.where(id:bot.id).update_all(original_user.except("id"))
  Room.where(id:room.id).update_all(original_room.except("id"))
  bot.update_columns(input[:user_attrs]) if input[:user_attrs]
  room.update_columns(input[:room_attrs]) if input[:room_attrs]
  Agent.where(id:agent.id).update_all(original.except("id"))
  AgentEvent.where(agent_id:agent.id).delete_all
  agent.reload.update_columns(input.fetch(:attrs, {})) unless input.fetch(:attrs, {}).empty?
  if input[:activity]
    agent.agent_events.create!(event_type:"mention",room_id:268197001,message_id:Message.first.id,outcome:"delivered")
  end
  Agent.where(id:agent.id).delete_all if input[:no_agent]
  Current.user = User.find(users.fetch(input[:viewer].to_sym))
  bot.reload
  controller = AgentProfileGoldenController.new
  controller.set_request!(ActionDispatch::Request.new(AgentProfileGoldenController.renderer.new(http_host:"campfire.test",https:false,"rack.session"=>{},"action_dispatch.content_security_policy_nonce_generator"=>->(_){"NONCE"}).send(:env_for_request)))
  controller.set_response!(ActionDispatch::Response.new)
  view = controller.view_context
  view.assign("user"=>bot,"dnd_allowed"=>false)
  html = view.render(template:"users/show",layout:false)
  input.merge(room_id:room.id,viewer_id:Current.user.id, html:html, nav:view.content_for(:nav).to_s)
end
puts JSON.pretty_generate(reference: ENV.fetch('PARITY_REFERENCE_SHA'), now:Time.current.iso8601, agent_id:agent.id, rows:rows)
warn "Rails agent profile oracle: #{rows.size} complete HTML/nav states; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
