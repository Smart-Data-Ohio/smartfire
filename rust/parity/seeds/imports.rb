# Completed previews and a failed run exercise import UI without contacting Slack.
based_on "default"
at NOW
workspace = SlackWorkspace.create!(client_id: "parity-slack-client", client_secret: "parity-offline-slack-secret", configured_by: user(:david), team_id: "PARITY", team_name: "Parity workspace", team_domain: "parity")
label :slack_imports, :workspace, SlackImport.create!(slack_workspace: workspace, user: user(:david), kind: "workspace", mode: "dry_run", status: "completed", started_at: NOW - 2.hours, finished_at: NOW - 1.hour, stats: { "conversations" => [], "samples" => [] })
label :slack_imports, :personal, SlackImport.create!(slack_workspace: workspace, user: user(:david), kind: "personal", mode: "dry_run", status: "completed", started_at: NOW - 2.hours, finished_at: NOW - 1.hour, stats: { "conversations" => [], "samples" => [] })
label :slack_imports, :failed, SlackImport.create!(slack_workspace: workspace, user: user(:david), kind: "workspace", mode: "import", status: "failed", error: "Recorded permission error", started_at: NOW - 2.hours, finished_at: NOW - 1.hour)
