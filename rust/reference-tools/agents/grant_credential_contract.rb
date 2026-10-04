require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16) do
  agent=Agent.find(773018776);human=User.find(127326141);room=Room.find(486777696)
  agent.agent_grants.delete_all
  grant=lambda do |attributes={}|
    row=AgentGrant.new({agent:agent,granted_by:human,capability:"post_messages"}.merge(attributes));row.valid?;row.errors.to_hash
  end
  grants={blank:grant.call(capability:""),unknown:grant.call(capability:"launch_missiles"),dm_room:grant.call(capability:"dm_anyone",room:room),missing_room:grant.call(room_id:0),missing_nonzero_room:grant.call(room_id:-1),missing_agent:grant.call(agent_id:0),missing_granter:grant.call(granted_by_id:0),dm_workspace:grant.call(capability:"dm_anyone")}
  existing=agent.agent_grants.create!(capability:"post_messages",room:room,granted_by:human)
  grants[:duplicate]=grant.call(room:room)
  grants[:other_scope]=grant.call
  existing.revoke!;first=existing.revoked_at;travel 1.second;existing.revoke!
  grants[:revoked_stamp_unchanged]=existing.revoked_at==first
  grants[:regrant]=grant.call(room:room)
  credential=lambda do |attributes={}|
    row=AgentCredential.new({agent:agent,created_by:human,name:"Test",token_digest:"ws11-public-digest",token_last_four:"disp"}.merge(attributes));row.valid?;row.errors.to_hash
  end
  credentials={blank_name:credential.call(name:""),blank_digest:credential.call(token_digest:""),blank_display:credential.call(token_last_four:""),missing_agent:credential.call(agent_id:0),missing_creator:credential.call(created_by_id:0)}
  existing=agent.agent_credentials.create!(created_by:human,name:"Existing",token_digest:"ws11-public-digest",token_last_four:"disp")
  credentials[:duplicate]=credential.call
  generated,secret=AgentCredential.create_with_secret!(agent:agent,created_by:human,name:"Generated")
  credentials[:generated]={secret_length:secret.length,digest_matches:generated.token_digest==AgentCredential.digest(secret),display_from_digest:generated.token_last_four==generated.token_digest[0,4],plaintext_stored:generated.attributes.values.include?(secret)}
  puts JSON.pretty_generate(reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],grants:grants,credentials:credentials)
end
