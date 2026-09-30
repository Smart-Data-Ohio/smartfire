# Reviewer minimal fixtures shared by the Rails checks and Rust oracle generator.
module Ws10ReviewFixtures
  def self.nested_mail(depth)
    raw = "From: outside@example.com\r\nTo: nobody@mail.test\r\n"
    depth.times { |i| raw << "Content-Type: multipart/mixed; boundary=b#{i}\r\n\r\n--b#{i}\r\n" }
    raw << "Content-Type: text/plain\r\n\r\nHello\r\n"
    (depth - 1).downto(0) { |i| raw << "--b#{i}--\r\n" }
    raw
  end

  def self.replay_mail
    "From: outside@example.com\r\nTo: room-token@mail.test\r\nSubject: Replay\r\n\r\nOnce only"
  end
end
