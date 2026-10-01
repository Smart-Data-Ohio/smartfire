require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
require Rails.root.join("db/migrate/20260916000000_create_agents")
ApplicationJob.queue_adapter = :test
ActiveRecord::Migration.verbose = false
travel_to Time.utc(2026, 3, 2, 16) do
  agent = Agent.find(773018776)
  actor = User.find(127326141)
  AgentCredential.delete_all
  AgentGrant.delete_all
  credential = agent.agent_credentials.create!(name: "Original", token_digest: "ws11-public-cleanup-digest", token_last_four: "disp", created_by: actor)
  grant = agent.agent_grants.create!(capability: "read_messages", granted_by: actor)
  results = {}
  results[:credential_same_stamp] = credential.tap { |c| c.update!(name: "Original") }.updated_at == credential.created_at
  travel 1.second
  credential.update!(name: "Renamed", expires_at: 1.hour.from_now)
  results[:credential_update] = credential.attributes.slice("name", "expires_at", "revoked_at", "updated_at")
  invalid = credential; invalid.name = " "
  invalid.valid?; results[:credential_invalid] = invalid.errors.to_hash
  credential.reload
  grant.update!(capability: "post_messages")
  results[:grant_update] = grant.attributes.slice("capability", "room_id", "revoked_at", "updated_at")
  other = agent.agent_grants.create!(capability: "react", granted_by: actor)
  other.assign_attributes(capability: "post_messages")
  other.valid?; results[:grant_duplicate] = other.errors.to_hash
  other.reload.revoke!
  other.update!(capability: "post_messages")
  results[:revoked_update_allowed] = other.revoked? && other.capability == "post_messages"
  credential.destroy!
  grant.destroy!
  results[:rows_destroyed] = [AgentCredential.exists?(credential.id), AgentGrant.exists?(grant.id)]
  # Destroy runs Agent's declared dependents, while budget notices/steps are not declared there.
  agent.agent_credentials.create!(name:"Dependent", token_digest:"ws11-dependent-digest", token_last_four:"disp", created_by:actor)
  approval=agent.agent_approvals.create!(action:"deploy",summary:"Cleanup")
  agent.agent_events.create!(event_type:"github_action_completed",outcome:"delivered")
  agent.agent_slash_commands.create!(room:Room.find(486777696),name:"cleanup",description:"cleanup")
  agent.destroy!
  results[:agent_destroy] = {
    agent: Agent.exists?(agent.id), credentials: AgentCredential.where(agent_id:agent.id).count,
    grants: AgentGrant.where(agent_id:agent.id).count, commands: AgentSlashCommand.where(agent_id:agent.id).count,
    events: AgentEvent.where(agent_id:agent.id).count, approvals: AgentApproval.where(agent_id:agent.id).count,
    inbox: ActivityItem.where(source_type:"AgentApproval",source_id:approval.id).count
  }
  Agent.delete_all
  CreateAgents.new.backfill_agents_for_existing_bots
  results[:backfill] = {same_users: Agent.pluck(:user_id).sort == User.where(role: :bot).ids.sort, workspace: Agent.all.all?(&:workspace?), ownerless: Agent.all.all?{|a| a.owner_id.nil?}, statuses: Agent.distinct.pluck(:status)}
  begin
    CreateAgents.new.backfill_agents_for_existing_bots
    results[:backfill_again] = "succeeded"
  rescue ActiveRecord::RecordNotUnique
    results[:backfill_again] = "RecordNotUnique"
  end
  puts JSON.pretty_generate({reference_pin:"d7c7de92",results:results}.as_json)
end
