//! Initials-avatar SVG bytes for the signed legacy avatar endpoint.
pub fn initials_svg(user_id: i64, initials: &str) -> String {
    let color = crate::helpers::avatar_background_color(user_id);
    let initials_escaped = crate::helpers::escape(initials);
    let adjust = if initials.chars().count() >= 3 {
        "textLength=\"85%\" lengthAdjust=\"spacingAndGlyphs\""
    } else {
        ""
    };
    format!(
        r##"<svg version="1.1" xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"
  viewBox="0 0 512 512" class="avatar" aria-hidden="true">
  <defs>
    <clipPath id="porthole">
      <circle cx="50%" cy="50%" r="50%" />
    </clipPath>
  </defs>

  <g>
    <rect width="100%" height="100%" rx="50" fill="{color}" />

    <text x="50%" y="50%" fill="#FFFFFF"
      text-anchor="middle" dy="0.35em"
      {adjust}
      font-family="-apple-system, BlinkMacSystemFont, Segoe UI, Roboto, Helvetica, Arial, sans-serif"
      font-size="230"
      font-weight="800"
      letter-spacing="-5">
      {initials}
    </text>
  </g>
</svg>
"##,
        initials = initials_escaped
    )
}
