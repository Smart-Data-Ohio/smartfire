class Rooms::FavoritesController < ApplicationController
  include RoomScoped

  def create
    @membership.favorite!
    head :ok
  end

  def destroy
    @membership.unfavorite!
    head :ok
  end

  # Move the favourite to an absolute 0-based position (see
  # Membership#move_favorite_to); out-of-range positions clamp.
  def update
    @membership.move_favorite_to(params[:position])
    head :ok
  end
end
