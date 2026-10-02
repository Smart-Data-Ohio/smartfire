# Native message children used by the complete room compositor; actual frozen Rails partials.
require 'json'
Rails.application.routes.default_url_options[:host]='campfire.test'
ActionCable.server.define_singleton_method(:broadcast) {|*_|}
Current.user=User.find(127326141)
rows=[]
[Rooms::Closed,Rooms::Direct].each_with_index do |klass,index|
  room=klass.create_for({id:9600+index,name:klass==Rooms::Direct ? nil : 'Quotes <&>',creator:Current.user},users:[Current.user,User.find(149087659)])
  (klass==Rooms::Direct ? [false] : [false,true]).each_with_index do |threaded,n|
    source=Message.create!(room:room,creator:Current.user,client_message_id:"ws13-source-#{index}-#{n}",markdown_source:'< & > '+('quotation '*35))
    if threaded
      thread=ChannelThread.create!(room:room,creator:Current.user,name:'Quote thread')
      source.update!(thread:thread)
    end
    quoting=Message.create!(room:room,creator:Current.user,client_message_id:"ws13-quoting-#{index}-#{n}",markdown_source:'Quoted message')
    reference=MessageReference.create!(message:quoting,referenced_message:source)
    html=ApplicationController.renderer.new(http_host:'campfire.test').render(partial:'messages/message_links/cards',locals:{message:quoting})
    rows << {input:{reference_id:reference.id,room_id:room.id,source_room_id:room.id,client_message_id:quoting.client_message_id,author:source.creator.name,room_label:ApplicationController.helpers.viewer_neutral_room_label(room),created_at:source.created_at.iso8601(6),plain_text:source.plain_text_body,path:URI.parse(Nokogiri::HTML.fragment(html).at_css(".message-quote__jump")["href"]).request_uri},html:html}
  end
end
puts JSON.pretty_generate({reference_pin:'d7c7de92',cases:rows})
