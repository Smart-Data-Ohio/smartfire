module Users::ProfilesHelper
  def profile_form_with(model, **params, &)
    form_with \
      model: @user, url: user_profile_path, method: :patch,
      data: { controller: "form" },
      **params,
      &
  end

  def profile_form_submit_button
    tag.button class: "btn btn--reversed center txt-large", type: "submit" do
      image_tag("check.svg", aria: { hidden: "true" }, size: 20) +
      tag.span("Save changes", class: "for-screen-reader")
    end
  end

  # IANA identifiers as values (auto-detect stores those), friendly Rails
  # names as labels, de-duplicated: several Rails zones share one
  # identifier, and one instant needs only one option.
  def profile_time_zone_choices
    ActiveSupport::TimeZone.all.map { |zone| [ zone.to_s, zone.tzinfo.identifier ] }
      .uniq { |(_, identifier)| identifier }
  end

  # The option matching the stored value: IANA identifiers match as-is,
  # legacy Rails names map to their identifier, and anything unknown (or
  # blank) leaves the "Not set" prompt selected.
  def profile_time_zone_value(stored)
    return nil if stored.blank?

    ActiveSupport::TimeZone[stored]&.tzinfo&.identifier
  end

  def web_share_session_button(url, title, text, &)
    tag.button class: "btn", hidden: true, data: {
      controller: "web-share", action: "web-share#share",
      web_share_url_value: url,
      web_share_text_value: text,
      web_share_title_value: title
    }, &
  end
end
