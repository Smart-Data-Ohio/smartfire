module Users::PresenceHelper
  PRESENCE_LABELS = {
    "online" => "Online",
    "idle" => "Idle",
    "dnd" => "Do not disturb",
    "offline" => "Offline",
    "agent" => "Agent"
  }.freeze

  def presence_label(presence)
    PRESENCE_LABELS.fetch(presence.to_s, presence.to_s.humanize)
  end

  # Effective presence for one user: agents keep their self-reported live
  # status, everyone else folds the manual setting over the live lease
  # state. Pass preloaded lease_states ({id => :online/:idle}) to avoid a
  # query when rendering many users.
  def display_presence_for(user, lease_states = nil)
    if user.bot? && user.agent
      agent = user.agent
      (agent.suspended_at.nil? && agent.last_seen_at.present?) ? :agent : :offline
    else
      lease_state = (lease_states || WorkspacePresenceLease.presence_by_user_id([ user.id ]))[user.id] || :offline
      user.effective_presence(lease_state)
    end
  end

  # The presence dot painted over an avatar. Other branches (the profile
  # card) can render this anywhere a user appears.
  def presence_dot_tag(user, presence: nil, lease_states: nil, **options)
    presence ||= display_presence_for(user, lease_states)

    tag.span role: "img", aria: { label: presence_label(presence) },
      class: [ "avatar__presence", options.delete(:class) ], data: { presence: presence.to_s }, **options
  end

  # Dot plus custom status text, as one badge. The profile card branch can
  # render this with `render_user_status_badge(user)`; pass a preloaded
  # presence to skip the lease lookup.
  def render_user_status_badge(user, presence: nil, lease_states: nil)
    render "users/statuses/badge", user:, presence: presence || display_presence_for(user, lease_states)
  end

  def user_theme
    theme = Current.user&.theme
    User::StatusSettings::THEMES.include?(theme) ? theme : "system"
  end

  def theme_color_scheme_meta_content
    case user_theme
    when "light" then "light"
    when "dark" then "dark"
    else "light dark"
    end
  end
end
