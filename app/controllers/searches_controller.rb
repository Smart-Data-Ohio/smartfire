class SearchesController < ApplicationController
  before_action :set_messages, only: :index

  def index
    @query = display_query if display_query.present?
    @recent_searches = Current.user.searches.ordered
    @return_to_room = last_room_visited
  end

  def create
    Current.user.searches.record(display_query)
    redirect_to searches_url(q: display_query)
  end

  def clear
    Current.user.searches.destroy_all
    redirect_to searches_url
  end

  private
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
      @messages = Message.none
      @has_more_older = false
      return if query.blank?

      scope = Current.user.reachable_messages.search(query)
      scope = scope.before(search_cursor) if params[:before].present?

      ids = scope.reorder(created_at: :desc, id: :desc)
        .limit(Message::PAGE_SIZE + 1).pluck(:id)
      @has_more_older = ids.size > Message::PAGE_SIZE
      page_ids = ids.first(Message::PAGE_SIZE)

      @messages = Current.user.reachable_messages.with_rendering_details
        .where(id: page_ids).ordered.order(:id).to_a
    end

    def search_cursor
      Current.user.reachable_messages.find(params[:before])
    end

    # The FTS5 MATCH expression. Every token is quoted as a phrase, so
    # operator words (AND, OR, NOT), trailing operators and quote
    # characters are searched literally instead of raising
    # "fts5: syntax error" or silently becoming a boolean query. Tokens
    # are word-character runs, so they cannot contain phrase syntax and
    # need no escaping. Porter stemming still applies inside phrases;
    # there was no prefix behaviour to keep (the expression never
    # appended `*`).
    def query
      display_query.to_s.scan(/[[:word:]]+/).map { |token| %("#{token}") }.join(" ").presence
    end

    # The human-readable query for display, history and redirects.
    def display_query
      params[:q].to_s.gsub(/[^[:word:]]/, " ").squish.presence
    end
end
