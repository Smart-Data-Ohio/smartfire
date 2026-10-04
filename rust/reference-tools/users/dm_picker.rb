require 'json'
require 'digest'
Rails.logger=ActiveSupport::Logger.new($stderr)
JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/users/dm-picker-source-hashes.json'))).each {|path,hash|raise "source drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest==hash}
class DmPickerGoldenController < Rooms::DirectsController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
user=User.find(127326141)
Current.reset;Current.user=user
renderer=DmPickerGoldenController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{})
cases=%w[seed starred escaped empty].map do |name|
  result=nil
  ActiveRecord::Base.transaction do
    UserStar.where(user_id:user.id).delete_all
    UserStar.create!(user:user,starred_user:User.find(149087659)) if name=='starred'
    User.create!(id:9100000001,name:'Renée <&> Dupont',email_address:'picker@example.test') if name=='escaped'
    User.where.not(id:user.id).update_all(status:1) if name=='empty'
    users=User.active.includes(:agent).with_attached_avatar.ordered.where.not(id:user.id).to_a
    starred=user.starred_ids_among(users.map(&:id));users=users.partition{|u|starred.include?(u.id)}.flatten
    input=users.map{|u|{id:u.id,name:u.name,role:u.role,status:u.status,starred:starred.include?(u.id),agent:!!u.agent}}
    result={name:name,people:input,html:renderer.render(template:'rooms/directs/new',layout:false,assigns:{users:users,starred_ids:starred})}
    raise ActiveRecord::Rollback
  end
  result
end
puts JSON.pretty_generate(reference: ENV.fetch('PARITY_REFERENCE_SHA'),picker:cases)
warn "Rails DM picker oracle: #{cases.size} complete picker bodies; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
