module SearchesHelper
  def search_results_tag(&)
    tag.div id: "search-results", class: "messages searches__results", data: {
      controller: "search-results message-list",
      search_results_target: "messages",
      search_results_me_class: "message--me",
      search_results_threaded_class: "message--threaded",
      search_results_mentioned_class: "message--mentioned",
      search_results_formatted_class: "message--formatted"
    }, &
  end

  # The header search field keeps the current query on the results page
  # and starts empty everywhere else.
  def global_search_query
    params[:q].to_s.squish.presence if controller_name == "searches"
  end
end
