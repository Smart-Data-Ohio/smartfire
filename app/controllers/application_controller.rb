class ApplicationController < ActionController::Base
  include AllowBrowser, Authentication, Authorization, BlockBannedRequests, SetCurrentRequest, SetPlatform, SudoMode, TrackedRoomVisit, VersionHeaders
  # Separate include, so its hooks register after Authentication's: one
  # multi-module include registers callbacks in reverse include order,
  # which would run the zone hook before Current.user is set.
  include SetTimeZone
  # Separate include for the same reason: enforcement must run after
  # Authentication has restored the session and Current.user.
  include TwoFactorEnforcement
  include Turbo::Streams::Broadcasts, Turbo::Streams::StreamName
  include MessagePayloadHelper

  helper MessagePayloadHelper

  private
    def no_store_response!
      response.headers["Cache-Control"] = "no-store"
      response.headers["Pragma"] = "no-cache"
    end
end
