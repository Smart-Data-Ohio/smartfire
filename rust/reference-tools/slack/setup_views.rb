require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
ActiveJob::Base.queue_adapter=:test
ActionCable.server.instance_variable_set(:@pubsub,ActionCable::SubscriptionAdapter::Test.new(ActionCable.server))
ActiveRecord::Schema.verbose=false;load Rails.root.join('db/schema.rb')
travel_to Time.utc(2026,1,1,12)
Account.create!(name:'Slack oracle');user=User.create!(id:811,name:'Oracle <&>',role: :administrator)
class Ws16SetupGoldenController < ApplicationController
 def form_authenticity_token(form_options: {})
  action,method=form_options.values_at(:action,:method)
  action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
 end
end
controller=Ws16SetupGoldenController.new
controller.set_request!(ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'example.org','rack.input'=>StringIO.new)))
controller.set_response!(ActionDispatch::Response.new)
renderer=Ws16SetupGoldenController.renderer.new(http_host:'example.org',https:false,'rack.session'=>{})
cases=[{name:'empty'},{name:'saved',configured:true},{name:'saved_without_configurer',configured:true,no_configurer:true},
 {name:'connected',configured:true,connected:true},{name:'ready',configured:true,connected:true,team:true},
 {name:'rejected',configured:true,connected:true,rejected:'Token <revoked &>',team:true},{name:'rejected_blank',configured:true,connected:true,rejected:' '},
 {name:'no_token',configured:true,connected:true,no_token:true},{name:'active',configured:true,connected:true,active:true},
 {name:'invalid_new',invalid:true},{name:'invalid_existing',configured:true,invalid:true}]
rows=cases.map do |c|
 SlackImport.delete_all;SlackConnection.delete_all;SlackWorkspace.delete_all
 workspace=SlackWorkspace.create!(id:851,client_id:'fixture-client',client_secret:'fixture-secret',configured_by:c[:no_configurer] ? nil : user,team_id:c[:team] ? 'TFIXTURE' : nil,team_name:c[:team] ? 'Team <&>' : nil) if c[:configured]
 connection=SlackConnection.create!(slack_workspace:workspace,user:,slack_user_id:'UFIXTURE',access_token:c[:no_token] ? nil : 'fixture-user-grant',disconnected_reason:c[:rejected]) if c[:connected]
 active=SlackImport.create!(id:853,slack_workspace:workspace,user:,kind:'personal',mode:'dry_run',status:'queued') if c[:active]
 if c[:invalid]
  workspace||=SlackWorkspace.new;workspace.client_id='';workspace.valid?
 end
 manifest=Slack::AppManifest.to_json(base_url:'http://example.org')
 Current.reset;Current.user=user
 html=renderer.render(template:'accounts/slack_imports/show',layout:false,assigns:{workspace:,connection:,active_run:active,manifest_json:manifest})
 data={client_id:workspace&.client_id,configured:workspace&.app_configured? || false,configured_by:workspace&.configured_by&.name,team_name:workspace&.team_name&.presence,team_known:workspace&.team_id.present?,connection_exists:!!connection,connected:connection&.connected? || false,disconnected_reason:connection&.disconnected_reason&.presence,active_run:active && active.attributes.slice('id','kind','mode','status'),errors:workspace&.errors&.full_messages || [],manifest:}
 {name:c[:name],data:,html:}
end
Current.reset
File.write(File.join(ENV.fetch('PARITY_WORK'),'vectors/slack/setup_views.json'),JSON.pretty_generate({reference:'d7c7de92',source_sha256:Digest::SHA256.file(Rails.root.join('app/views/accounts/slack_imports/show.html.erb')).hexdigest,cases:rows})+"\n")
puts "Slack setup views: #{rows.length} complete Rails template bodies generated with shared deterministic CSRF inputs"
