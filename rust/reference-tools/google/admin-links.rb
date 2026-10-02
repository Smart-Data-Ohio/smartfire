require 'json'
require 'nokogiri'
require 'action_dispatch/testing/integration'
ActiveJob::Base.queue_adapter=:test
ActionController::Base.allow_forgery_protection=false
now=Time.utc(2026,3,2,16)
Time.define_singleton_method(:current) { now }
Rails.application.routes.default_url_options.merge!(host:'campfire.test',protocol:'http')
specs=[
 {name:'allow',actor:'david',target:'kevin',method:'post'},
 {name:'allow_again',actor:'david',target:'kevin',method:'post',allowed:true},
 {name:'unlink',actor:'david',target:'jz',method:'delete'},
 {name:'unlink_again',actor:'david',target:'jz',method:'delete',linked:false},
 {name:'member_allow',actor:'jz',target:'kevin',method:'post'},
 {name:'member_unlink',actor:'jz',target:'jz',method:'delete'},
 {name:'bot_target',actor:'david',target:'bender',method:'post'},
 {name:'admin_controls',actor:'david',method:'get'},
 {name:'member_controls',actor:'jz',method:'get'}
]
cases=[]
specs.each do |spec|
 GoogleIdentity.delete_all
 kevin=User.find_by!(name:'Kevin');jz=User.find_by!(name:'JZ')
 kevin.update_columns(email_address:'kevin@smartdata.net',email_self_changed_at:spec[:allowed] ? nil : now-86400,google_email_link_allowed:!!spec[:allowed])
 GoogleIdentity.create!(user:jz,subject:'google-sub-jz',email:'jz@smartdata.net',domain:'smartdata.net') unless spec[:linked]==false
 actor=User.find_by!(name:{'david'=>'David','jz'=>'JZ'}.fetch(spec[:actor]));target=spec[:target] && User.find_by!(name:{'kevin'=>'Kevin','jz'=>'JZ','bender'=>'Bender Bot'}.fetch(spec[:target]))
 session=Session.create!(user:actor,user_agent:'admin-links-fixture',ip_address:'127.0.0.1',two_factor_verified_at:now)
 req=ActionDispatch::Request.new(Rails.application.env_config.merge('HTTP_HOST'=>'campfire.test','rack.url_scheme'=>'http','REQUEST_METHOD'=>'GET'));jar=ActionDispatch::Cookies::CookieJar.build(req,{})
 jar.signed[:session_token]={value:session.token}
 client=ActionDispatch::Integration::Session.new(Rails.application);client.host! 'campfire.test'
 AuditLog.delete_all
 client.public_send(spec[:method],target ? "/account/users/#{target.id}/google_link" : '/account/edit',headers:{'HTTP_COOKIE'=>"session_token=#{URI.encode_www_form_component(jar[:session_token])}"})
 forms=Nokogiri::HTML(client.response.body).css('form').filter_map { |f| f['action'] if f['action']&.end_with?('/google_link') }
 cases << {spec:,actor_id:actor.id,target_id:target&.id,kevin_id:kevin.id,jz_id:jz.id,status:client.response.status,location:client.response.location,changed_at:kevin.reload.email_self_changed_at&.iso8601,allowed:kevin.google_email_link_allowed,identity_count:GoogleIdentity.where(user:jz).count,audits:AuditLog.order(:id).map { |a|a.attributes.slice('action','actor_id','target_id','target_type','target_label','details') },forms:}
end
class GoogleAdminHtmlController < ApplicationController
  self.allow_forgery_protection=true
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
# Integration requests release their CurrentAttributes execution state.
ActiveSupport::ExecutionContext.clear
Current.user=User.find_by!(name:'David')
renderer=GoogleAdminHtmlController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{})
control_rows=User.order(:id).map do |u|
  html=renderer.render(partial:'accounts/users/user',locals:{user:u})
  {id:u.id,name:u.name,email:u.email_address,role:u.role,status:u.status,google_identity_email:u.google_identity&.email,untrusted:u.email_self_changed_at.present? || !u.google_email_link_allowed?,forms:html.scan(/<form\b[^>]*\baction="[^"]*\/google_link"[\s\S]*?<\/form>/)}
end
puts JSON.pretty_generate({reference:'d7c7de92',cases:,control_rows:})
