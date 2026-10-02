require "json"
ActiveRecord::Base.logger = nil
user = User.first!
channel = Calendar::PushChannel.create!(user:, channel_id:"fixture-channel", token_digest:Calendar::PushChannel.digest("fixture-token"))
values = ["7", "  +7abc", "-7", "bad", "0", "9223372036854775808", "99999999999999999999999999999999999999", "0x10", "1_2", "\u00a07"]
cases = values.map do |number|
  channel.update_columns(last_message_number:0)
  begin
    won = channel.claim_notification!(number)
    {input:number, won:, saved:channel.reload.last_message_number}
  rescue => error
    {input:number, error:error.class.name}
  end
end
puts JSON.pretty_generate({reference:"d7c7de9264c63015be398001d7a1094e7695a6db", digest:Calendar::PushChannel.digest("fixture-token"), claims:cases})
