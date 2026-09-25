require "application_system_test_case"

class SearchRecentsClearTest < ApplicationSystemTestCase
  setup do
    sign_in "jz@37signals.com"
    users(:jz).searches.record("alpha")
    users(:jz).searches.record("beta")
  end

  # Regression: clearing redirected back to the search page, but the
  # redirected GET inherited Turbo's Turbo Stream Accept header and got the
  # "Load older results" stream instead of the page, so the recent search
  # links stayed on screen until the user left search and came back.
  test "clearing recent searches on the search page empties them without leaving" do
    visit searches_url
    assert_selector "a[href='#{searches_path(q: "alpha")}']", visible: true, wait: 10
    assert_selector "a[href='#{searches_path(q: "beta")}']", visible: true

    accept_confirm { click_button "Clear recent searches" }

    assert_no_selector "a[href='#{searches_path(q: "alpha")}']", visible: true, wait: 10
    assert_no_selector "a[href='#{searches_path(q: "beta")}']", visible: true
    assert_current_path searches_path
    assert_empty users(:jz).searches.reload
  end
end
