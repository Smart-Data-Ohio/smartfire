require 'digest'
ENV['INBOUND_EMAIL_DOMAIN'] = 'mail.test'
bot = User.active_bots.find_by!(name: 'Email')
raise bot.errors.full_messages.inspect unless bot.valid?
raise 'bot plaintext persisted' unless bot.bot_token.nil?
raise 'bot not scoped to one room' unless bot.memberships.count == 1
messages = Message.where(creator: bot).or(Message.where(markdown_source: "**Launch**\n\nFriday."))
raise 'wrong message count' unless messages.count == 2
messages.each do |message|
  raise message.errors.full_messages.inspect unless message.valid?
  raise 'message source missing' if message.markdown_source.blank?
  raise 'root message became thread' unless message.thread_id.nil?
  if message.attachment.attached?
    raise 'attachment bytes changed' unless message.attachment.download == 'file-bytes'
  end
end
inbounds = ActionMailbox::InboundEmail.all
raise 'wrong inbound count' unless inbounds.count == 3
inbounds.each do |inbound|
  raise 'raw blob missing' unless inbound.raw_email.attached?
  raise 'checksum changed' unless Digest::SHA1.hexdigest(inbound.source) == inbound.message_checksum
  raise 'wrong lifecycle' unless inbound.delivered? || inbound.bounced?
end
room = Room.find_by!(inbound_email_token: Room.find(messages.first.room_id).inbound_email_token)
raise room.errors.full_messages.inspect unless room.valid?
old_token = room.inbound_email_token
raise 'Rails rotation failed' if room.regenerate_inbound_email_token! == old_token
puts 'Rails rollback: Email bot, room, 2 messages, 3 inbound emails and attachment bytes validated'
