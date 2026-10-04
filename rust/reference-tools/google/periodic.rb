# Registered Rust task inventory, read from our Rails Periodic::Runner. The other
# tasks are owned by the other workstreams and are not registered on this branch.
require "json"
names = ["saved item reminders", "scheduled messages", "poll closing", "stuck rooms",
  "calendar push channels", "retention prune"]
runner = Periodic::Runner.new(reminders_interval: 17, retention_interval: 123)
tasks = runner.instance_variable_get(:@tasks).select { |task| names.include?(task.name) }
raise "missing Rails task" unless tasks.map(&:name).sort == names.sort
puts JSON.pretty_generate({reference: ENV.fetch("PARITY_REFERENCE_SHA"),
  tasks: tasks.map { |task| {name: task.name, seconds: task.interval.to_i} }})
