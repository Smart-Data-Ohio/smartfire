user = User.create!(name: "Timestamp fixture", email_address: "timestamp-fixture@smartdata.net", password: nil, role: :member)
identity = GoogleIdentity.create!(user:, subject: "timestamp-fixture-subject", email: user.email_address, domain: "smartdata.net")
raise "Rails creation timestamps differ" unless identity.created_at == identity.updated_at
identity.update!(email: "timestamp-fixture-changed@smartdata.net")
saved_at = identity.updated_at
raise "Rails loaded and persisted save timestamps differ" unless identity.reload.updated_at == saved_at
puts "Rails GoogleIdentity timestamp contract: one insert instant; loaded update instant matches persisted"
