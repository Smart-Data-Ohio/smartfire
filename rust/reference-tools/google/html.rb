require "json"
ENV["GOOGLE_CLIENT_ID"] = "test-client-id"
ENV["GOOGLE_CLIENT_SECRET"] = "FAKE-google-client-secret"
ENV["GOOGLE_SIGN_IN_DOMAINS"] = "smartdata.net,cnbssoftware.com,other.test"
class GoogleHtmlController < ApplicationController
  def form_authenticity_token(form_options: {})
    action, method = form_options.values_at(:action, :method)
    action && method ? "#{method.to_s.downcase}:#{action}" : "GLOBAL"
  end
end
renderer = GoogleHtmlController.renderer.new(http_host: "campfire.test", https: false, "rack.session" => {})
puts JSON.pretty_generate({reference:"d7c7de9264c63015be398001d7a1094e7695a6db", html:renderer.render(partial:"sessions/google_sign_in"), domains:Google::SignIn.allowed_domains.map { |d| "@#{d}" }.to_sentence})
