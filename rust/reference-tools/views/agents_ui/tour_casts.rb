# The stored-value DateTime.deserialize boundary used by the tour marker.
inputs=JSON.parse(File.read(File.join(ENV.fetch('PARITY_WORK'),'vectors/agent-tour-values.json')))
ActiveRecord::Base.logger=nil
cases=inputs.fetch('cases').map do |input|
 ActiveRecord::Base.connection.execute(input.fetch('sql'))
 value=User.find(127326141).tour_completed_at
 stored=value.respond_to?(:iso8601) ? ActiveRecord::Base.connection.send(:quoted_date,value) : value
 {name:input.fetch('name'),sql:input.fetch('sql'),stored:}
end
puts JSON.pretty_generate(reference:ENV.fetch("PARITY_REFERENCE_SHA"),cases:)
warn "Rails stored datetime casts: #{cases.size} values"
