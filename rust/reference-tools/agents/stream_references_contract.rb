# Actual pinned Rails reference callbacks; no mocked sync or mocked rows.
ApplicationJob.queue_adapter = :test
require 'json'
bot=User.find_by!(name:'Bender Bot');room=bot.rooms.find_by!(name:'All Talk')
m=room.messages.create!(creator:bot,streaming:true,markdown_source:'See https://example.test/some/page',client_message_id:'ws11-reference-probe')
result={start:m.link_embed_references.count}
m.update!(markdown_source:'See https://example.test/some/page and more')
result[:append]=m.link_embed_references.reload.count
m.finalize_stream!
result[:finalize]=m.link_embed_references.reload.count
m.finalize_stream!
result[:repeat]=m.link_embed_references.reload.count
quiet=room.messages.create!(creator:bot,streaming:true,markdown_source:'https://example.test/quiet-page',client_message_id:'ws11-quiet-reference-probe')
quiet.finalize_stream_quietly!
result[:quiet]=quiet.link_embed_references.reload.count
puts JSON.pretty_generate(result)
