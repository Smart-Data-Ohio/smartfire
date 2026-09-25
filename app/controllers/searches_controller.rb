class SearchesController < ApplicationController
  before_action :set_messages, only: :index

  def index
    @query = display_query if display_query.present?
    @recent_searches = Current.user.searches.ordered.limit(10)
    @return_to_room = last_room_visited

    # Only a "Load older results" window renders as a Turbo Stream.
    # Turbo form submissions (the header search, clearing recents) send
    # a Turbo Stream Accept header that the redirected GET inherits;
    # answering those with the prepend stream would leave the page as it
    # was, so everything else renders the page.
    render formats: :html if params[:before].blank?
  end

  def create
    if parsed_query.blank_query?
      redirect_back_or_to searches_url, notice: "Enter a word to search for."
    else
      Current.user.searches.record(display_query)
      redirect_to searches_url(q: display_query)
    end
  end

  # Clearing from the header dropdown or the results page empties every
  # recents list in place, wherever the user is; without Turbo it lands
  # back on the page it came from.
  def clear
    respond_to do |format|
      format.turbo_stream { clear_recent_searches }
      format.html do
        clear_recent_searches
        redirect_back_or_to searches_url
      end
    end
  end

  private
    # Inside the format branches, so a request for a format this action
    # can't answer gets its 406 before anything is deleted.
    def clear_recent_searches
      Current.user.searches.destroy_all
    end

    # Results page newest-first in fixed windows, with a "Load older
    # results" cursor on (created_at, id): the id half keeps
    # same-timestamp messages from skipping or repeating at page edges.
    # The window is selected as ids with LIMIT in SQL (plus one probe
    # row to learn whether older results exist), so a large result set
    # never instantiates more than a page; only the window is then
    # loaded with its rendering preloads, and displayed oldest-first.
    # The preloads stay deliberate: the results render through
    # messages/_message, which reads boosts and attachments either way,
    # and preloading costs ~12 queries against ~370 lazy ones.
    def set_messages
      @search_query = parsed_query
      @messages = Message.none
      @board_posts = []
      @work_threads = []
      @events = []
      @has_more_older = false
      return if @search_query.blank_query?

      scope = @search_query.apply_to_messages(Current.user.reachable_messages)
      scope = scope.before(search_cursor) if params[:before].present?

      ids = scope.reorder(created_at: :desc, id: :desc)
        .limit(Message::PAGE_SIZE + 1).pluck(:id)
      @has_more_older = ids.size > Message::PAGE_SIZE
      page_ids = ids.first(Message::PAGE_SIZE)

      @messages = Message::MentionPreloader.preload_for(
        Current.user.reachable_messages.with_rendering_details
          .where(id: page_ids).ordered.to_a
      )

      # Boards, work threads, and events search the operator-free text as
      # capped side sections on the first page only: older message
      # windows prepend without repeating them.
      if params[:before].blank?
        @board_posts = @search_query.board_posts_for(Current.user).to_a
        @work_threads = @search_query.work_threads_for(Current.user).to_a
        @events = @search_query.events_for(Current.user).to_a
      end
    end

    def search_cursor
      Current.user.reachable_messages.find(params[:before])
    end

    def parsed_query
      @parsed_query ||= SearchQuery.parse(params[:q])
    end

    # The human-readable query for display, history and redirects,
    # operators included.
    def display_query
      params[:q].to_s.squish.presence
    end
end
