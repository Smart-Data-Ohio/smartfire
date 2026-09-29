# `bin/rails db:fixtures:load` (activerecord's railties/databases.rake), plus the fixture classes
# test/test_helper.rb sets with `set_fixture_class`, which the rake task can't know: without them the
# Twitter fixtures resolve to no model and their associations stay unexpanded. Run by
# reference-tools/db/differential.sh.
require "active_record/fixtures"
require "active_support/testing/time_helpers"

# The Rust loader compares row for row, timestamps included, at the same frozen instant
# (fixtures_match_ruby_row_for_row). Without a freeze each fixture set takes its own Time.now.
if (now = ENV["CAMPFIRE_FIXTURES_NOW"].presence)
  extend ActiveSupport::Testing::TimeHelpers
  travel_to ActiveSupport::TimeZone["UTC"].parse(now), with_usec: true
end

fixtures_dir = ActiveRecord::Tasks::DatabaseTasks.fixtures_path
files = Dir[File.join(fixtures_dir, "**/*.{yml}")]
files.reject! { |f| f.start_with?(File.join(fixtures_dir, "files")) }
files.map! { |f| f[fixtures_dir.to_s.size..-5].delete_prefix("/") }

ActiveRecord::FixtureSet.create_fixtures(fixtures_dir, files,
  "twitter_posts" => Twitter::Post, "twitter_post_references" => Twitter::PostReference)
