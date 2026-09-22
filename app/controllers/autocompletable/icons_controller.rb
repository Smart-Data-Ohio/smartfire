class Autocompletable::IconsController < ApplicationController
  def index
    # The emoji picker's Custom tab lists every workspace icon; search
    # keeps its ranked top-8 across brands, customs, and emoji.
    @icons = if params[:custom].present?
      Icons.custom_icons
    else
      Icons.search(params[:q].presence || params[:query], limit: 8)
    end
  end
end
