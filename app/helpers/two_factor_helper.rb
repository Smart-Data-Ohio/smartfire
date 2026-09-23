module TwoFactorHelper
  # Renders the provisioning URI as an inline QR code. The secret stays
  # in this page's HTML: unlike link_to_zoom_qr_code it never travels
  # through the QR image URL (and its server logs).
  def two_factor_qr_code(uri)
    RQRCode::QRCode.new(uri).as_svg(viewbox: true, fill: :white, color: :black).html_safe
  end
end
