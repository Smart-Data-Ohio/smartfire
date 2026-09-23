# Administrator sessions expire after this much idle time, checked on every
# request and on Action Cable connect. Members keep the current
# never-expire lifetime. Override with ADMIN_SESSION_IDLE_TIMEOUT_DAYS
# (whole days; anything unparseable or under a day falls back to 7).
Rails.application.configure do
  days = ENV.fetch("ADMIN_SESSION_IDLE_TIMEOUT_DAYS", "7").to_i
  days = 7 if days < 1
  config.x.admin_session_idle_timeout = days.days
end
