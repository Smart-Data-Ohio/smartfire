require "active_support/testing/time_helpers"
extend ActiveSupport::Testing::TimeHelpers
ApplicationJob.queue_adapter=:test
travel_to Time.utc(2026,3,2,16) do
  cases={}
  room=Room.find(486777696);david=User.find(127326141);bot=User.find(394959859)
  helper=ApplicationController.helpers
  helper.extend ActionDispatch::Routing::UrlFor
  helper.extend Rails.application.routes.url_helpers
  helper.define_singleton_method(:default_url_options) {{host:"payload.test",protocol:"https"}}
  capture=->(name,message,viewer=david,base="payload.test") do
    Current.user=viewer
    helper.define_singleton_method(:default_url_options) {{host:base,protocol:"https"}}
    cases[name]=helper.message_payload(message.reload).as_json
  end
  capture.call(:rich_text,Message.find(136976342))
  markdown=room.messages.create!(creator:david,markdown_source:"Hello **world** & <x>\n\n```rust\nlet x = 1;\n```",client_message_id:"ws11-presenter")
  capture.call(:markdown,markdown)
  reply=room.messages.create!(creator:bot,markdown_source:"Answer",client_message_id:"ws11-presenter-reply",reply_to_message:markdown,reply_notify_author:true,forwarded_from_message:markdown,forwarded_at:Time.current,forward_note:"From elsewhere")
  reply.drive_attachments.create!(file_id:"ws11-public-drive-file")
  capture.call(:reply_forward_drive,reply)
  reply.update_columns(reply_to_message_id:nil,reply_target_deleted_at:Time.current)
  capture.call(:deleted_reply,reply)
  thread=ChannelThread.create!(room:room,creator:david,name:"Presenter chat",parent_message:markdown)
  ThreadMembership.join!(thread,david)
  capture.call(:root_human,markdown)
  capture.call(:root_bot,markdown,bot)
  threaded=thread.messages.create!(room:room,creator:bot,markdown_source:"Thread reply",client_message_id:"ws11-presenter-thread",streaming:true)
  capture.call(:thread_bot,threaded,bot)
  capture.call(:thread_human,threaded,david,"other.test")
  puts JSON.pretty_generate(reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:cases)
end
