require "json"
Rails.logger = ActiveSupport::Logger.new($stderr)
Rails.logger.level = Logger::FATAL
class FizzyProfileGoldenController < Users::ProfilesController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
user = User.find(127326141)
Current.user = user
inputs = [
  {name:"missing"},
  {name:"usable", token:"ws8br2-fizzy-fixture-token"},
  {name:"blank", token:" "},
  {name:"blank_reason", token:" ", reason:" "},
  {name:"unreadable", token:"bad-ciphertext", unreadable:true},
  {name:"rejected", token:"ws8br2-fizzy-fixture-token", reason:"Fizzy rejected <& token"}
]
renderer = FizzyProfileGoldenController.renderer.new(http_host:"campfire.test",https:false,"rack.session"=>{})
rows = inputs.map do |input|
  FizzyConnectedAccount.where(user_id:user.id).delete_all
  if input[:token]
    account = FizzyConnectedAccount.create!(user:user, fizzy_account_id:"fixture-account", fizzy_account_name:"Team <& Co", fizzy_user_name:"Person <& Co", access_token:input[:token], disconnected_reason:input[:reason])
    FizzyConnectedAccount.connection.execute("UPDATE fizzy_connected_accounts SET access_token=#{FizzyConnectedAccount.connection.quote(input[:token])} WHERE id=#{account.id}") if input[:unreadable]
  end
  user.reload
  html = renderer.render(partial:"users/profiles/fizzy_connection",locals:{user:user})
  input.merge(html:html, reason_after:FizzyConnectedAccount.find_by(user_id:user.id)&.disconnected_reason)
end
puts JSON.pretty_generate(reference:"d7c7de92",rows:rows)
warn "Rails Fizzy profile oracle: #{rows.size} complete fragments and token-usability side effects; reference d7c7de92"
