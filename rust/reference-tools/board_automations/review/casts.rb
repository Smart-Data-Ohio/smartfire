require 'json'
require 'digest'
require 'active_support/testing/time_helpers'
include ActiveSupport::Testing::TimeHelpers
travel_to Time.utc(2026,3,2,16)
JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'), 'reference-tools/board_automations/dispatch-source-hashes.json'))).each do |file, hash|
  raise "source drift: #{file}" unless Digest::SHA256.file(Rails.root.join(file)).hexdigest == hash
end
conn=ActiveRecord::Base.connection
conn.execute("UPDATE rooms SET type='Rooms::Board' WHERE id=486777696")
conn.execute('DELETE FROM board_sla_rules')
values=JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'reference-tools/board_automations/review/casts.json')))+['0d42','+0D42','-0d42','1.','1.e2',' 1. ',' 1.e2 ']
rows=values.map do |input|
 r=BoardSlaRule.new(room_id:486777696,work_status:'planned',nudge_after_minutes:input,escalate_after_minutes:'10')
 valid=r.valid?
 {input:input,valid:valid,cast:r.nudge_after_minutes&.to_s,errors:r.errors.to_hash,full_messages:r.errors.full_messages}
end
puts JSON.generate(rows:rows)
warn "REVIEW Rails decimal model: #{rows.size} inputs; casts and complete validation errors"
