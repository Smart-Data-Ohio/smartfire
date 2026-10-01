# Rails callback ordering, tag transactions and full-relation board filters.
require "json"
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
ActiveJob::Base.queue_adapter = :test
CAPTURE = []
module BoardCallbackProbe
  %w[prepend replace remove].each do |action|
    define_method("broadcast_#{action}_to") do |*streamables, target:, **options|
      CAPTURE << {action:action,target:target.to_s,column:options.dig(:locals,:dom_suffix) == :board_column_row}
    end
  end
end
ChannelThread.prepend(BoardCallbackProbe)
board = Room.find(699448332)
creator = User.find(127326141)
queries = [
  {status:"open",owner:"anyone",tag:"",page:1}, {status:"done",owner:"anyone",tag:"",page:1},
  {status:"all",owner:"me",tag:"",page:1}, {status:"all",owner:"agents",tag:"",page:1},
  {status:"all",owner:"394959859",tag:" Rust ",page:1}, {status:"all",owner:"unknown",tag:"release",page:1},
  {status:"all",owner:"anyone",tag:"missing",page:1}, {status:"all",owner:"0",tag:"",page:1},
  {status:"all",owner:"9999999999999999999999999999",tag:"",page:1}
].map do |args|
  args.merge(ids:ChannelThread.board_posts_for(board,**args,viewer:creator).to_a.map(&:id))
end
steps=[]
thread=nil
[
  ["create", -> { thread=ChannelThread.create!(room:board,creator:creator,name:"Created",work_status:"planned"); thread.id }],
  ["name_twice", -> { ChannelThread.transaction {thread.update!(name:"First");thread.update!(name:"Second")} }],
  ["tags", -> { thread.tag_names="bug,api";thread.save! }],
  ["replace_tags", -> { thread.tag_names="api,docs";thread.save! }],
  ["same_tags", -> { thread.tag_names="docs,api";thread.save! }],
  ["last_change_not_row", -> { ChannelThread.transaction {thread.update!(name:"No broadcast");thread.update!(auto_archive_after_minutes:60)} }],
  ["rollback", -> { ChannelThread.transaction {thread.tag_names="rollback";thread.save!;raise ActiveRecord::Rollback} }],
  ["destroy", -> {thread.reload.destroy!}]
].each do |name,operation|
  CAPTURE.clear
  operation.call
  steps << {name:,thread_id:thread.id,callbacks:CAPTURE.dup,tags:ThreadTag.where(channel_thread_id:thread.id).order(:name).pluck(:name)}
end
puts JSON.pretty_generate(reference:"d7c7de92",queries:,tag_counts:ChannelThread.board_tag_counts(board),steps:)
warn "Rails board domain oracle: #{queries.size} filters and #{steps.size} committed/rolled-back callback sequences; reference d7c7de92"
