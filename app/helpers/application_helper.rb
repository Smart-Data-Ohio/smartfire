module ApplicationHelper
  def page_title_tag
    tag.title @page_title || "Smartfire"
  end

  def current_user_meta_tags
    unless Current.user.nil?
      safe_join [
        tag(:meta, name: "current-user-id", content: Current.user.id),
        tag(:meta, name: "current-user-name", content: Current.user.name)
      ]
    end
  end

  # The saved zone for the timezone controller, or an empty marker when
  # the member explicitly chose "Not set": the controller reports only
  # while the tag has no content at all, so the marker suppresses a
  # detection the server would ignore anyway.
  def current_user_time_zone_meta_content
    Current.user.time_zone.presence || ("" if Current.user.time_zone_explicit?)
  end

  # Manual DND and the DND presence mute sounds outright; quiet hours send
  # their window and zone so the sound controller re-evaluates the
  # time-based gate on every play without a reload.
  def notification_sound_meta_tags
    return unless Current.user

    tags = []
    if Current.user.manual_dnd_active? || Current.user.presence_setting == "dnd"
      tags << tag.meta(name: "notification-dnd", content: "muted")
    end
    if Current.user.quiet_hours_enabled? && Current.user.quiet_hours_start_minute && Current.user.quiet_hours_end_minute
      tags << tag.meta(name: "quiet-hours",
        content: "#{Current.user.quiet_hours_start_minute}-#{Current.user.quiet_hours_end_minute}")
      tags << tag.meta(name: "quiet-hours-zone", content: Current.user.time_zone_or_default)
    end
    safe_join(tags)
  end

  def custom_styles_tag
    if custom_styles = Current.account&.custom_styles
      tag.style(custom_styles.to_s.html_safe, data: { turbo_track: "reload" })
    end
  end

  def body_classes
    [ @body_class, admin_body_class, account_logo_body_class ].compact.join(" ")
  end

  def link_back
    back_url = request.referrer
    back_url = root_path if back_url.nil? || back_url == request.url
    link_back_to back_url
  end

  def link_back_to(destination)
    link_to destination, class: "btn" do
      image_tag("arrow-left.svg", aria: { hidden: "true" }, size: 20) +
      tag.span("Go Back", class: "for-screen-reader")
    end
  end

  private
    def admin_body_class
      "admin" if Current.user&.can_administer?
    end

    def account_logo_body_class
      "account-has-logo" if Current.account&.logo&.attached?
    end
end
