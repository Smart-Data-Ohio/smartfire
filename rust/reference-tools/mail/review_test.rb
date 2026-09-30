require '/rails/test/test_helper'
require_relative 'review_fixtures'

class Ws10ReviewRegressionTest < ActionMailbox::TestCase
  test 'review deep MIME bounces before the body is split' do
    [64, 128, 2000].each do |depth|
      raw = Ws10ReviewFixtures.nested_mail(depth)
      split_calls = 0
      trace = TracePoint.new(:call) { |event| split_calls += 1 if event.defined_class == Mail::Body && event.method_id == :split! }
      inbound = nil
      assert_no_difference -> { Message.count } do
        trace.enable { inbound = receive_inbound_email_from_source(raw) }
      end
      assert_predicate inbound.reload, :bounced?
      assert_equal 0, split_calls
      puts "Rails MIME depth=#{depth} bytes=#{raw.bytesize} status=#{inbound.status} posts=0 body_splits=#{split_calls}"
    end
  end

  test 'review NBSP is not Ruby regex whitespace or strip whitespace' do
    original = ENV['INBOUND_EMAIL_AUTHSERV_ID']
    ENV['INBOUND_EMAIL_AUTHSERV_ID'] = 'mx.mail.test'
    mailbox = RoomMailbox.new(nil)
    ["mx.mail.test; dkim=pass\u00a0header.d=example.com", "\u00a0mx.mail.test; dkim=pass header.d=example.com"].each do |header|
      parsed = Mail.read_from_string("From: member@example.com\r\nTo: room-token@mail.test\r\nAuthentication-Results: #{header}\r\n\r\nHello")
      mailbox.define_singleton_method(:mail) { parsed }
      assert_not mailbox.send(:authenticated_sender?, 'member@example.com')
    end
  ensure
    ENV['INBOUND_EMAIL_AUTHSERV_ID'] = original
  end
end
