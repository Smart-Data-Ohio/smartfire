require "json"
require "digest"
Rails.logger=ActiveSupport::Logger.new($stderr)
JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"),"reference-tools/users/icons-source-hashes.json"))).each { |path,hash| raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest==hash }
class IconsGoldenController < Accounts::IconsController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
Current.user=User.find(127326141)
def icon(name:"acme",title:"Acme Corp",file:"clean.svg")
  WorkspaceIcon.new(name:name,title:title,creator:Current.user).tap do |i|
    i.image.attach(io:File.open(Rails.root.join("test/fixtures/files/workspace_icons",file)),filename:file) if file
  end
end
WorkspaceIcon.destroy_all
inputs=Dir.children(Rails.root.join("test/fixtures/files/workspace_icons")).sort.map {|file| {file:file} }
inputs += ["a","x"*33,"has space","has-dash","has.dot","UPPER!",":emoji:","openai","gpt","tada","  Acme_Corp  ",""].map {|name|{name:name} }
inputs += ["","x","x"*60,"x"*61,"é"*60," " ].map {|title|{title:title} }
inputs += [{file:nil}]
cases=inputs.map { |input| i=icon(**input); {input:input,name:i.name,title:i.title,valid:i.valid?,errors:i.errors.full_messages,content_type:i.image.attached? ? i.image.blob.content_type : nil} }
def render_icons(new_icon)
  controller=IconsGoldenController.new
  controller.set_request!(ActionDispatch::Request.new(IconsGoldenController.renderer.new(http_host:"campfire.test",https:false,"rack.session"=>{},"action_dispatch.content_security_policy_nonce_generator"=>->(_){"NONCE"}).send(:env_for_request)))
  controller.set_response!(ActionDispatch::Response.new)
  view=controller.view_context
  view.assign({workspace_icons:WorkspaceIcon.ordered.with_attached_image.includes(:creator),workspace_icon:new_icon}.stringify_keys)
  html=view.render(template:"accounts/icons/index",layout:false)
  {html:html,nav:view.content_for(:nav).to_s}
end
pages=[{name:"empty",**render_icons(WorkspaceIcon.new)}]
first=icon; first.id=11001;first.save!
second=icon(name:"zeta",title:'Zeta <&"');second.id=11002;second.save!
pages << {name:"populated",**render_icons(WorkspaceIcon.new)}
invalid=icon(name:"openai",title:"",file:"script.svg");invalid.valid?
pages << {name:"invalid",**render_icons(invalid)}
Current.user.update_columns(name:'<b>Creator & "</b>')
first.update_columns(title:'<b>Icon & "</b>')
second.update_columns(title:'<script>title()</script>')
markup_icon=WorkspaceIcon.new(name:'<b>name & "</b>',title:'<i>Form title & "</i>')
pages << {name:"review_markup",icons:WorkspaceIcon.ordered.map {|i|{id:i.id,name:i.name,title:i.title,creator_name:i.creator.name}},form:{name:markup_icon.name,title:markup_icon.title},**render_icons(markup_icon)}
duplicate=icon(name:"ACME");duplicate.valid?
cases << {input:{name:"ACME",duplicate:true},name:duplicate.name,title:duplicate.title,valid:false,errors:duplicate.errors.full_messages,content_type:duplicate.image.blob.content_type}
puts JSON.pretty_generate(reference:"d7c7de92",cases:cases,pages:pages)
warn "Rails icons oracle: #{cases.size} validation cases, #{pages.size} complete HTML/nav cases; reference d7c7de92"
