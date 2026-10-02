# Public configuration, including Ruby's Unicode blankness; no OAuth secret is required.
require 'json'
keys=%w[GOOGLE_CLIENT_ID GOOGLE_PICKER_API_KEY GOOGLE_CLOUD_PROJECT_NUMBER]
base=keys.zip(%w[test-client-id test-picker-key 123456789012]).to_h
specs=[base,base.merge('GOOGLE_CLOUD_PROJECT_NUMBER'=>'not-a-number'),base.transform_values { |v|" #{v} " }]
keys.each { |key|[nil,'',' ',"\t\n","\u00a0","\u2003"].each { |value|specs<<base.merge(key=>value) } }
rows=specs.map do |values|
  keys.each { |key| values[key].nil? ? ENV.delete(key) : ENV[key]=values[key] }
  ENV.delete('GOOGLE_CLIENT_SECRET')
  configured=Google::Picker.configured?
  {values:,configured:,public:configured ? {client_id:Google::Picker.client_id,api_key:Google::Picker.api_key,project_number:Google::Picker.project_number} : nil}
end
puts JSON.pretty_generate({reference:'d7c7de92',rows:})
