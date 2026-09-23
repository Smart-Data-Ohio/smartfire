# Records the first-run tour as seen. Skipping and finishing both land
# here: either way the tour never auto-starts again, and the help menu
# restarts it on demand without clearing the stamp.
class Users::ToursController < ApplicationController
  def update
    Current.user.touch(:tour_completed_at)
    head :no_content
  end
end
