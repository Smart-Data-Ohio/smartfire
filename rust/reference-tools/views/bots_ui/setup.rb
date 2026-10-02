# Exact bot UI pages from pinned Smartfire Rails, with fixtures and fixed CSRF/nonce values.
require "json"
require "ostruct"

ActiveRecord::Base.logger = nil

# `fixtures :all` with test_helper.rb's fixture classes.
require "active_record/fixtures"
fixtures = Rails.root.join("test/fixtures")
ActiveRecord::FixtureSet.create_fixtures(fixtures,
  Dir[fixtures.join("**/*.yml")].map { |path| path.delete_prefix("#{fixtures}/").delete_suffix(".yml") },
  { "twitter_posts" => Twitter::Post, "twitter_post_references" => Twitter::PostReference })

OUT = "/rails/storage/db/goldens.json"
HOST = "campfire.test"
USER_AGENT = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36"

class GoldenController < ApplicationController
  # "GLOBAL" for csrf_meta_tags, "<method>:<action>" for a form's per-form token.
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end

class GoldenSearchesController < SearchesController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end

  def self.controller_name = "searches"
end

def user(email) = User.find_by!(email_address: email)

def renderer_env(flash: {}, query: nil)
  {
    http_host: HOST,
    https: false,
    "HTTP_USER_AGENT" => USER_AGENT,
    "QUERY_STRING" => query.to_s,
    "rack.session" => {},
    "action_dispatch.content_security_policy" => Rails.application.config.content_security_policy,
    "action_dispatch.content_security_policy_nonce_generator" => ->(_request) { "NONCE" },
    "action_dispatch.request.flash_hash" => ActionDispatch::Flash::FlashHash.new(flash.stringify_keys)
  }
end

def render_with(controller: GoldenController, user: nil, flash: {}, query: nil, **options)
  Current.reset
  Current.user = user
  controller.renderer.new(renderer_env(flash:, query:)).render(**options)
ensure
  Current.reset
end



