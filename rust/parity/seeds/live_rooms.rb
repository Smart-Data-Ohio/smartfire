# Recorded voice participants and stage stream. No LiveKit/gateway calls are made.
based_on "default"
{
  "LIVEKIT_URL" => "ws://gateway.parity.invalid",
  "LIVEKIT_INTERNAL_URL" => "http://livekit.parity.invalid",
  "LIVEKIT_API_KEY" => "parity-offline-key",
  "LIVEKIT_API_SECRET" => "parity-offline-secret-at-least-32-characters",
  "LIVEKIT_GATEWAY_SECRET" => "parity-offline-gateway-secret"
}.each { |key, value| label :reference_env, key, value; ENV[key] = value }
at NOW
%w[voice stage].each do |name|
  room = room(name)
  membership = room.memberships.find_by!(user_id: id_for(:users, :jason))
  membership.update!(stage_role: "speaker", hand_raised_at: nil) if name == "stage"
  HuddleGrant.create!(user: user(:jason), membership: membership, room: room,
    session: Session.find(id_for(:sessions, :jason)), identity: "parity-#{name}-jason", room_name: "room_#{room.id}",
    last_issued_at: NOW, last_seen_at: NOW, stage_role: membership.stage_role)
end
stage = room(:stage)
member = stage.memberships.find_by!(user_id: id_for(:users, :jason))
Stream.create!(room: stage, membership: member, user: user(:jason), quality: "1080p15", started_at: NOW - 10.minutes)
Event.find(id_for(:events, :launch_party)).update!(venue: stage)
