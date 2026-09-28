module Slack
  # Generates the internal Slack app manifest the administrator pastes at
  # api.slack.com/apps (Create New App -> From a manifest). A user-token
  # importer needs no bot user, no events, and no bot scopes: only the
  # OAuth redirect URL and the user scopes the connection requires.
  #
  # Manifest schema per https://docs.slack.dev/reference/app-manifest
  class AppManifest
    DISPLAY_NAME = "Smartfire Import"

    class << self
      def generate(base_url:)
        {
          "display_information" => { "name" => DISPLAY_NAME },
          "settings" => { "org_deploy_enabled" => false },
          "oauth_config" => {
            "redirect_urls" => [ "#{base_url}/slack/oauth/callback" ],
            "scopes" => { "user" => OAuth::USER_SCOPES }
          }
        }
      end

      def to_json(base_url:)
        JSON.pretty_generate(generate(base_url:))
      end
    end
  end
end
