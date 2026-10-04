require "json"
ActiveJob::Base.queue_adapter = :test
room=Room.find(486777696)
user=User.find(127326141)
venue=Rooms::Voice.create!(name:"Workshop",creator:user)
venue.memberships.create!(user:, involvement: :everything)
cases=[ ["utc",{time_zone:"UTC"}], ["ny",{time_zone:"America/New_York"}], ["fallback-end",{time_zone:"UTC",ends_at:nil}], ["description-venue",{time_zone:"UTC",description:"Bring snacks",venue:}], ["blank-description",{time_zone:"UTC",description:"  "}] ]
puts JSON.pretty_generate({reference:ENV.fetch("PARITY_REFERENCE_SHA"),ids:[[7,9],[127326141,149087659],[-1,1],[9223372036854775807,1]].map{|e,u|{event_id:e,user_id:u,id:Calendar::EntrySync.google_event_id_for(e,u)}},cases:cases.map{|name,attrs|event=Event.create!({room:,organizer:user,title:"Party",starts_at:Time.utc(2026,9,23,10),ends_at:Time.utc(2026,9,23,12)}.merge(attrs));{name:,event:event.attributes.slice("id","room_id","title","description","starts_at","ends_at","time_zone"),venue:event.venue&.attributes&.slice("id","name"),origin:(Rails.application.routes.default_url_options[:host] ? "#{Rails.application.routes.default_url_options[:protocol] || 'http'}://#{Rails.application.routes.default_url_options[:host]}" : nil),payload:Calendar::EntrySync.new(event,user).send(:payload)}}})
