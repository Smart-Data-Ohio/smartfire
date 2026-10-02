require "json"
require "uri"
controller=Rooms::DriveRecipientsController.new
samples=[nil,"1",[1," 2 ","01",1],[],["mail@example.test"],[nil],[true],[[1]],[{}],["-1"],["1e2"],["1_2"]]
puts JSON.pretty_generate({reference:"d7c7de9264c63015be398001d7a1094e7695a6db",email_pattern:URI::MailTo::EMAIL_REGEXP.source,email_options:URI::MailTo::EMAIL_REGEXP.options,
  emails:["a@example.test","a@localhost","a+b@external.test","a..b@external.test"," a@example.test","a@invalid_underscore.test","a@x","😀@example.test","a@-bad.test","a@example.test\n","\"a\"@example.test","a@sub.example.test"].map{|v|{input:v,eligible:URI::MailTo::EMAIL_REGEXP.match?(v)}},
  selections:samples.map{|v|controller.params=ActionController::Parameters.new(user_ids:v);{input:v,ids:controller.send(:normalized_user_ids)}}})
