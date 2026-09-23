class ApplicationMailbox < ActionMailbox::Base
  routing(/room-(.+)@/i => :room)
end
