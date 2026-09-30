require "json"
require "digest"
Rails.logger=ActiveSupport::Logger.new($stderr)
ledger=JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"),"reference-tools/users/account-views-source-hashes.json")))
ledger.each { |path,hash| raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest==hash }
class AccountViewsGoldenController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
def render_account(viewer,template,assigns={},partial:false)
  Current.user=viewer
  controller=AccountViewsGoldenController.new
  controller.set_request!(ActionDispatch::Request.new(AccountViewsGoldenController.renderer.new(http_host:"campfire.test",https:false,"rack.session"=>{},"action_dispatch.content_security_policy_nonce_generator"=>->(_){"NONCE"}).send(:env_for_request)))
  controller.set_response!(ActionDispatch::Response.new)
  view=controller.view_context
  view.assign(assigns.stringify_keys)
  html=view.render(**(partial ? {partial:template,locals:assigns} : {template:template,layout:false}))
  {html:html,nav:view.content_for(:nav).to_s,footer:view.content_for(:footer).to_s,last_room_id:controller.send(:last_room_visited)&.id,app_version:Rails.application.config.app_version}
ensure
  Current.reset
end
david=User.find(127326141); kevin=User.find(712064548); bender=User.find(394959859)
rows=[
 ["self",david,david,{}], ["member",david,kevin,{}], ["viewer_member",kevin,david,{}],
 ["linked",david,kevin,{identity:true}], ["self_linked",kevin,kevin,{identity:true}],
 ["email_changed",david,kevin,{self_changed:true,allowed:true}], ["allowed",david,kevin,{allowed:true}],
 ["blank_email",david,kevin,{email:" "}], ["banned",david,kevin,{status:2}],
 ["deactivated",david,kevin,{status:1}], ["bot",david,bender,{}],
 ["two_factor",david,kevin,{two_factor:true}], ["escaped",david,kevin,{identity:true,name:'Name <&"',email:'member+fixture@campfire.test'}]
].map do |name,viewer,user,setup|
  result=nil
  ActiveRecord::Base.transaction(requires_new:true) do
    attrs={google_email_link_allowed:setup.fetch(:allowed,false),email_self_changed_at:setup[:self_changed] ? Time.current : nil}
    attrs[:email_address]=setup[:email] if setup.key?(:email)
    attrs[:name]=setup[:name] if setup.key?(:name)
    attrs[:status]=setup[:status] if setup.key?(:status)
    user.update_columns(**attrs)
    user.google_identity&.destroy!
    user.create_google_identity!(subject:"ws8br2-row",email:attrs.fetch(:email_address,user.email_address),domain:"campfire.test") if setup[:identity]
    if setup[:two_factor]
      user.two_factor_credential&.destroy!
      user.create_two_factor_credential!(secret:Array.new(32,"A").join,confirmed_at:Time.current)
    end
    result={name:name,viewer_id:viewer.id,user_id:user.id,setup:setup,**render_account(viewer.reload,"accounts/users/user",{user:user.reload},partial:true)}
    raise ActiveRecord::Rollback
  end
  result
end
pages=[david,kevin].map do |viewer|
  users=(viewer.can_administer? ? User.where(status:[:active,:banned]) : User.active).ordered.without_bots
  administrators,members=users.partition(&:administrator?)
  page=GearedPagination::Recordset.new(users,per_page:500).page(1)
  {viewer_id:viewer.id,account_id:Account.first.id,join_code:Account.first.join_code,administrator_ids:administrators.map(&:id),member_ids:members.map(&:id),**render_account(viewer,"accounts/edit",{account:Account.first,administrators:administrators,members:members,page:page})}
end
invites=[david,kevin].map { |viewer| {viewer_id:viewer.id,join_code:Account.first.join_code,**render_account(viewer,"accounts/invite",{},partial:true)} }
styles=[nil,"/* é <&> */\nbody { color: red; }"].map do |css|
  account=Account.first; account.assign_attributes(custom_styles:css)
  {custom_styles:css,**render_account(david,"accounts/custom_styles/edit",{account:account})}
end
puts JSON.pretty_generate(reference:"d7c7de92",rows:rows,pages:pages,invites:invites,styles:styles)
warn "Rails account views oracle: #{rows.size} rows, #{pages.size} settings bodies/navs/footers, #{invites.size} invites, #{styles.size} CSS bodies; reference d7c7de92"
