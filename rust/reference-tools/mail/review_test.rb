require '/rails/test/test_helper'
require_relative 'review_fixtures'
require 'json'

class Ws10ReviewRegressionTest < ActionMailbox::TestCase
  test 'review multipart fixtures post the Rails body and attachment' do
    original = ENV['INBOUND_EMAIL_DOMAIN']
    ENV['INBOUND_EMAIL_DOMAIN'] = 'mail.test'
    room = rooms(:designers)
    token = room.regenerate_inbound_email_token!
    output = JSON.parse(File.read('/tools/corpus/multipart.json')).map do |label, raw|
      inbound = nil
      assert_difference -> { room.messages.count }, 1 do
        inbound = receive_inbound_email_from_source(raw.gsub('room-token@mail.test', "room-#{token}@mail.test"))
      end
      assert_predicate inbound.reload, :delivered?
      message = room.messages.order(:id).last
      {label: label, raw: raw, source: message.markdown_source,
       filename: message.attachment.attached? ? message.attachment.filename.to_s : nil,
       bytes: message.attachment.attached? ? Base64.strict_encode64(message.attachment.download) : nil}
    end
    File.write('/out/multipart.json', JSON.pretty_generate(output) + "\n")
  ensure
    ENV['INBOUND_EMAIL_DOMAIN'] = original
  end

  test 'review room-addressed deep MIME records the reference outcome' do
    original = ENV['INBOUND_EMAIL_DOMAIN']
    ENV['INBOUND_EMAIL_DOMAIN'] = 'mail.test'
    room = rooms(:designers)
    token = room.regenerate_inbound_email_token!
    raw = Ws10ReviewFixtures.nested_mail(4000, fixed_width: true)
      .sub('nobody@mail.test', "room-#{token}@mail.test")
    before = room.messages.count
    begin
      inbound = receive_inbound_email_from_source(raw)
    rescue SystemStackError
      # Mail 2.9.1 has no MIME depth cap; Message#all_parts recurses until the
      # configured Ruby VM stack is exhausted. Record this Rails defect explicitly.
      assert_equal before, room.messages.count
      inbound = ActionMailbox::InboundEmail.order(:id).last
      puts "Rails room MIME depth=4000 status=#{inbound.reload.status} error=SystemStackError posts=0 vm_stack=#{ENV.fetch('RUBY_THREAD_VM_STACK_SIZE', 'default')}"
    else
      assert_equal before + 1, room.messages.count
      assert_predicate inbound.reload, :delivered?
      assert_equal "From outside@example.com\n\nHello", room.messages.order(:id).last.markdown_source
      puts "Rails room MIME depth=4000 status=#{inbound.status} posts=1 source=#{room.messages.order(:id).last.markdown_source.inspect} vm_stack=#{ENV.fetch('RUBY_THREAD_VM_STACK_SIZE', 'default')}"
    end
  ensure
    ENV['INBOUND_EMAIL_DOMAIN'] = original
  end

  test 'review deep MIME bounces before the body is split' do
    [64, 128, 2000, 4000].each do |depth|
      raw = Ws10ReviewFixtures.nested_mail(depth, fixed_width: depth == 4000)
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
