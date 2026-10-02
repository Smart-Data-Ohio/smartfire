# The pinned controller's save/destroy and later AuditLog.record! are distinct
# transactions. Exercise both actual actions with a failing SQLite audit insert.
labels = JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'parity/.seed/default/labels.json')))
bot = User.find(labels.fetch('users.bender'))
Current.user = User.find(labels.fetch('users.david'))
Github::WriteClient.define_singleton_method(:authenticated_login) { |_token| 'boundary-machine' }
ActiveRecord::Base.connection.execute("CREATE TRIGGER reject_github_audit BEFORE INSERT ON audit_logs WHEN NEW.action LIKE 'agent.github.%' BEGIN SELECT RAISE(ABORT,'fixture audit rejection'); END;")
controller = Accounts::Bots::GithubConnectionsController.new
controller.params = ActionController::Parameters.new(access_token: 'boundary-fixture-token')
controller.instance_variable_set(:@bot, bot)
controller.define_singleton_method(:edit_account_bot_path) { |_bot| '/fixture-edit' }
%w[create destroy].each do |action|
  begin
    controller.public_send(action)
    raise "#{action}: expected audit write failure"
  rescue ActiveRecord::StatementInvalid => error
    raise unless error.message.include?('fixture audit rejection')
  end
  account = GithubConnectedAccount.find_by(user: bot)
  raise "link did not remain committed" if action == 'create' && !(account&.access_token == 'boundary-fixture-token')
  raise "unlink did not remain committed" if action == 'destroy' && account
  raise 'an audit was unexpectedly stored' if AuditLog.where(action: 'agent.github.' + (action == 'create' ? 'connect' : 'disconnect')).exists?
end
puts 'Rails GitHub audit boundary: link and unlink remain committed after a rejected audit insert; 2 actions passed'
