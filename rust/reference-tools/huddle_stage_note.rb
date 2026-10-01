require 'json'
require 'active_support/testing/time_helpers'
class HuddleStageNoteOracle
  include ActiveSupport::Testing::TimeHelpers
  def run
    ActiveRecord::Schema.verbose=false
    load Rails.root.join('db/schema.rb')
    ActiveRecord::FixtureSet.create_fixtures(Rails.root.join('test/fixtures'),%w[accounts users rooms memberships])
    travel_to Time.utc(2026,1,1,12)
    user=User.find(ActiveRecord::FixtureSet.identify('david'))
    room=Rooms::Stage.create!(id:9001,name:'WS13 Stage',creator:user)
    note=room.messages.create!(id:1200000001,client_message_id:'ws13-stage-note-fixture',creator:user,system_note:true,body:'The stage ended because the last host left.')
    html=ApplicationController.render(partial:'messages/message',locals:{message:note})
    puts JSON.pretty_generate({reference_pin:'d7c7de92',now:Time.current.to_i,room:room.attributes,message:note.attributes,body:note.body.to_plain_text,html:html})
  ensure
    travel_back
  end
end
HuddleStageNoteOracle.new.run
