# Public Picker configurations and complete composer bytes from pinned Rails only.
require 'json'
require 'digest'
raise 'Picker source drift' unless Digest::SHA256.file(Rails.root.join('app/models/google/picker.rb')).hexdigest == 'b7ad42ff5682480b341f5b366f64f668a65ec8e593d4320b79ec33a77709b0e8'
class WS8brPickerComponentsController < ApplicationController
  def form_authenticity_token(form_options: {})
    action,method=form_options.values_at(:action,:method)
    action && method ? "#{method.to_s.downcase}:#{action}" : 'GLOBAL'
  end
end
renderer=WS8brPickerComponentsController.renderer.new(http_host:'campfire.test',https:false,'rack.session'=>{})
actor=User.find(127326141); room=Room.find(486777696)
config={ 'GOOGLE_CLIENT_ID'=>'public-client<&>', 'GOOGLE_PICKER_API_KEY'=>'public-picker-key', 'GOOGLE_CLOUD_PROJECT_NUMBER'=>'12345' }
rows=(0...8).map do |bits|
  config.keys.each_with_index {|key,index| bits[index]==1 ? ENV[key]=config[key] : ENV.delete(key)}
  Current.reset;Current.user=actor
  parts=JSON.parse(renderer.render(inline:'<% render template: "rooms/show" %><%= {footer: content_for(:footer)}.to_json.html_safe %>',layout:false,assigns:{room:room,messages:[],ooo_notice_members:[]}))
  {input:config.select{|key,_|ENV.key?(key)},configured:Google::Picker.configured?,composer:parts['footer']}
end
Current.reset
puts JSON.pretty_generate(reference_pin: ENV.fetch('PARITY_REFERENCE_SHA'),picker_cases:rows)
warn "Rails Picker components: #{rows.size} public configurations and complete root composers"
