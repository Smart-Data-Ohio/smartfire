require "digest"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"),"reference-tools/agents/work-services-source-hashes.json"))).each do |file,hash|
  raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest==hash
end
travel_to Time.utc(2026,3,2,16)
ApplicationJob.queue_adapter=:test
human=User.find(127326141); agent=Agent.find(773018776)
board=Rooms::Board.create_for({name:"Rules",creator:human},users:[human,User.find(773523953),agent.user])
rows=[]
[
 ["human","bug",773523953,[]], ["normalize","  Bug ",773523953,[]], ["agent","bug",agent.user_id,[]],
 ["blank"," ",773523953,[]], ["format","Not A Tag!",773523953,[]], ["cap","x"*31,773523953,[]],
 ["boundary","x"*30,773523953,[]], ["duplicate","BUG",773523953,["duplicate"]],
 ["other_board","bug",773523953,["other_board"]], ["channel","bug",human.id,["channel"]],
 ["missing_room","bug",human.id,["missing_room"]], ["missing_assignee","bug",0,[]],
 ["missing_creator","bug",773523953,["missing_creator"]], ["outside","bug",712064548,[]],
 ["deactivated","bug",773523953,["deactivated"]], ["suspended","bug",agent.user_id,["suspended"]],
 ["no_post","bug",agent.user_id,["no_post"]], ["no_read","bug",agent.user_id,["no_read"]]
].each do |name,tag,assignee_id,flags|
  ActiveRecord::Base.transaction do
    AgentGrant.delete_all
    %w[read_messages post_messages].each { |cap|AgentGrant.create!(agent:agent,room:board,capability:cap,granted_by:human) }
    BoardTagAssignment.create!(room:board,tag:"bug",assignee:User.find(773523953),created_by:human) if flags.include?("duplicate") || flags.include?("other_board")
    User.find(773523953).update!(status: :deactivated) if flags.include?("deactivated")
    agent.update!(suspended_at:Time.current) if flags.include?("suspended")
    %w[no_post no_read].each do |flag|
      AgentGrant.where(agent:agent,capability:flag=="no_post" ? "post_messages" : "read_messages").update_all(revoked_at:Time.current) if flags.include?(flag)
    end
    room=flags.include?("channel") ? Room.find(486777696) : board
    room=Rooms::Board.create_for({name:"Other",creator:human},users:[human,User.find(773523953)]) if flags.include?("other_board")
    rule=BoardTagAssignment.new(room_id:flags.include?("missing_room") ? 0 : room.id,tag:,assignee_id:,created_by_id:flags.include?("missing_creator") ? 0 : human.id)
    valid=rule.valid?
    rows << {name:,tag:,assignee_id:,flags:,expected:{valid:,tag:rule.tag,errors:rule.errors.full_messages}}
    raise ActiveRecord::Rollback
  end
  agent.reload
end
puts JSON.pretty_generate(reference:"d7c7de92",rows:)
warn "Rails board tag assignment oracle: #{rows.size} validation cases"
