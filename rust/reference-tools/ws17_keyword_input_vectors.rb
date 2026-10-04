# Real params coercion and keyword-list writer, including compound textarea values.
ActiveRecord::Base.logger = nil
user = User.find_by!(email_address: "david@37signals.com")
values = [nil, false, true, 12, "Deploy\n deploy \nlaunch", ["Deploy", "launch"], [12, false], [["deploy", "freeze"]], {x: "deploy"}, [{x: "deploy"}], [["x\n", "é", nil, true, 12]], [["\u0000\u001b\u007f"+'#{x}', "\u2028"]]]
rows = values.map do |input|
  user.keyword_alerts.delete_all
  user.errors.clear
  normalized = ActionDispatch::Request::Utils.normalize_encode_params(JSON.parse(JSON.generate(input)))
  params = ActionController::Parameters.new(user: {keyword_alerts: normalized})
  value = params.dig(:user, :keyword_alerts)
  error = lines = saved = nil
  begin
    lines = Array(value).map(&:to_s)
    saved = user.replace_keyword_alerts(value)
  rescue => exception
    error = exception.class.name
  end
  {input:, lines:, error:, saved:, phrases: user.keyword_alerts.order(:phrase).pluck(:phrase), errors: user.errors.map { |e| [e.attribute.to_s, e.message] }}
end
puts JSON.generate(reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], rows:)
