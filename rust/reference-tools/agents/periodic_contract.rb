# Preserve Rails ordering for every scheduler task currently implemented by Rust.
# The full Rails roster is captured too; other task implementations retain their owners.
runner = Periodic::Runner.new(reminders_interval:17, retention_interval:123)
all = runner.instance_variable_get(:@tasks).map { |t| { name:t.name, seconds:t.interval.to_i } }
implemented = ["saved item reminders", "scheduled messages", "poll closing", "stuck rooms", "stranded agent webhooks", "clear plaintext bot tokens", "retention prune", "streaming messages"]
puts JSON.pretty_generate(reference_pin:"d7c7de92", all_tasks:all, implemented_tasks:all.select { |t| implemented.include?(t[:name]) })
