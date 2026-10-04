# Our actual Rails policy/status readers at the current reference pin. No network or persisted writes.
# PARITY_NAMESPACE=ws17 PARITY_IMAGE=campfire-reference \
#   parity/bin/reference runner --seed default reference-tools/ws17_vectors.rb
require "active_support/testing/time_helpers"
include ActiveSupport::Testing::TimeHelpers

travel_to(Time.utc(2026, 9, 30, 12)) do
  policy_vectors = []
  %w[room_message thread_message reminder huddle huddle_join].product(
    ["absent", nil, "invisible", "nothing", "muted", "mentions", "everything"],
    [nil, "nothing", "mentions", "everything"], (0..7).to_a, [false, true]
  ).each do |kind, room_involvement, thread_involvement, bits, quiet|
    recipient = User.new(name: "Recipient", role: :member, status: :active, dnd_enabled: quiet)
    room = Membership.new(involvement: room_involvement) unless room_involvement == "absent"
    thread = ThreadMembership.new(involvement: thread_involvement) if thread_involvement
    input = { kind:, room: room_involvement, thread: thread_involvement,
      mentioned: bits.anybits?(1), reply: bits.anybits?(2), keyword: bits.anybits?(4), quiet: }
    policy = Notifications::Policy.new(recipient:, kind: kind.to_sym, room_membership: room,
      thread_membership: thread, mentioned: input[:mentioned], reply_to_recipient: input[:reply],
      keyword_matched: input[:keyword], dnd_exception: false)
    policy_vectors << input.merge(push: policy.push?, sound: policy.sound?, inbox: policy.inbox_event_type)
  end

  status_vectors = []
  base = Time.utc(2026, 9, 30, 12)
  cases = [
    {}, { dnd_enabled: true }, { dnd_enabled: true, dnd_until: base.iso8601 },
    { dnd_enabled: true, dnd_until: (base + 1).iso8601 }, { presence_setting: "dnd" },
    { custom_status_text: "Working", custom_status_emoji: "💻" },
    { custom_status_text: "Expired", custom_status_expires_at: base.iso8601 },
    { custom_status_text: " ", custom_status_emoji: "" },
    { ooo_until: (base + 86400).iso8601, ooo_note: "Vacation" },
    { ooo_until: base.iso8601, ooo_note: "Expired" },
    { ooo_until: (base + 86400).iso8601, presence_setting: "invisible", custom_status_text: "Own status" },
    { ooo_until: (base + 86400).iso8601, ooo_notify_enabled: true },
    { meeting_status_enabled: true }, { meeting_status_enabled: true, meeting_dnd_enabled: true },
    { meeting_status_enabled: true, custom_status_text: "Own status" },
    { meeting_status_enabled: true, presence_setting: "invisible" },
    { meeting_status_enabled: true, dnd_enabled: true },
    { meeting_status_enabled: false, meeting_dnd_enabled: true },
    { ooo_calendar_enabled: true }, { ooo_calendar_enabled: true, ooo_notify_enabled: true },
    { ooo_calendar_enabled: true, ooo_until: (base + 3600).iso8601, ooo_note: "Own note" }
  ]
  pairs = [[(base - 60).iso8601, (base + 86400 * 2).iso8601], ["bad", "bad"], nil, 42]
  cases.each_with_index do |attrs, index|
    user = User.new(name: "Recipient", role: :member, status: :active, **attrs)
    user.meeting_cache = Calendar::MeetingCache.new(busy_intervals: pairs, ooo_intervals: pairs)
    policy = Notifications::Policy.new(recipient: user, kind: :reminder)
    status_vectors << { name: "status-#{index}", now: base.iso8601, attrs:, busy: pairs, ooo: pairs,
      dnd: user.dnd_active?, meeting: user.in_meeting?, out_of_office: user.out_of_office?,
      custom: user.custom_status_display, status: user.status_text_display, push: policy.push?,
      presence: %i[online idle offline].map { |p| user.effective_presence(p).to_s } }
  end
  # Minute gates at the skipped spring hour and both copies of the autumn hour.
  ["America/New_York", "Eastern Time (US & Canada)", "America/Los_Angeles", "Asia/Kolkata", "UTC"].each do |zone|
    %w[2026-03-08T06:59:59Z 2026-03-08T07:00:00Z 2026-03-08T07:59:59Z 2026-03-08T08:00:00Z
      2026-11-01T05:15:00Z 2026-11-01T06:15:00Z 2026-11-01T07:00:00Z].each do |instant|
      [[60, 120], [180, 240], [1320, 420], [0, 0]].each do |start_minute, end_minute|
        now = Time.iso8601(instant)
        attrs = { time_zone: zone, quiet_hours_enabled: true, quiet_hours_start_minute: start_minute, quiet_hours_end_minute: end_minute }
        user = User.new(name: "Recipient", **attrs)
        policy = Notifications::Policy.new(recipient: user, kind: :reminder, now:)
        status_vectors << { name: "#{zone}-#{instant}-#{start_minute}-#{end_minute}", now: instant, attrs:, busy: [], ooo: [],
          dnd: user.dnd_active?(now:), meeting: user.in_meeting?(now:), out_of_office: user.out_of_office?(now:),
          custom: user.custom_status_display(now:), status: user.status_text_display(now:), push: policy.push?,
          presence: %i[online idle offline].map { |p| user.effective_presence(p).to_s } }
      end
    end
  end
  coercions = [nil, true, false, [], [1], {"x" => 1}, 1, 1.9, -1.9,
    "", " ", "1", "12abc", "+12", "-12", "001", "010", "08", "0x10", "0b10", "0o10", "0d10", "1_2", "1__2", "_12", "12_", "0x_10", " 12 ", "1.0", "0_10", "0x", "9223372036854775808", "-9223372036854775808", "12\n", "\u00a012"].map do |input|
    { input:, output: Integer(input, exception: false) }
  end
  task = Periodic::Runner.new.instance_variable_get(:@tasks).find { |t| t.name == "presence leases" }
  presence_task = { name: task.name, seconds: task.interval.to_i }
  puts JSON.generate({ presence_task:, integer_coercions: coercions, reference: ENV.fetch("PARITY_REFERENCE_SHA")[0, 8], now: base.iso8601, policies: policy_vectors, statuses: status_vectors })
end
