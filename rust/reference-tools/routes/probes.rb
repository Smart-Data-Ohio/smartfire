# Rails-owned expectations for positional, explicit nil and default parameter semantics.
require "json"
rows = JSON.parse(File.read("/work/reference-tools/routes/probes.json"))
helpers = Rails.application.routes.url_helpers
rows.each do |row|
  begin
    row["path"] = helpers.public_send("#{row.fetch("name")}_path", *row.fetch("args"), **row.fetch("options").transform_keys(&:to_sym))
  rescue ActionController::UrlGenerationError => error
    row["error"] = error.class.name
  end
end
File.write("/output/default-probes.json", JSON.pretty_generate(rows) + "\n")
puts "Rails default parameter probes: #{rows.count { |r| r.key?("path") }} paths, #{rows.count { |r| r.key?("error") }} errors"
