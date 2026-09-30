# Reviewer minimal fixtures shared by the Rails checks and Rust oracle generator.
module Ws10ReviewFixtures
  def self.nested_mail(depth, fixed_width: false)
    raw = +"From: outside@example.com\r\nTo: nobody@mail.test\r\n"
    boundaries = depth.times.map { |i| fixed_width ? format('b%08d', i) : "b#{i}" }
    boundaries.each { |boundary| raw << "Content-Type: multipart/mixed; boundary=#{boundary}\r\n\r\n--#{boundary}\r\n" }
    raw << "Content-Type: text/plain\r\n\r\nHello\r\n"
    boundaries.reverse_each { |boundary| raw << "--#{boundary}--\r\n" }
    raw
  end

  def self.replay_mail
    "From: outside@example.com\r\nTo: room-token@mail.test\r\nSubject: Replay\r\n\r\nOnce only"
  end

  def self.retry_mail
    "From: outside@example.com\r\nTo: room-token@mail.test\r\nSubject: Retry\r\n\r\nRetry body"
  end
end
