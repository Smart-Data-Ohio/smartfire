class ApplicationMailbox < ActionMailbox::Base
  routing(/room-(.+)@/i => :room)
  routing(:all => :bounce)
end
