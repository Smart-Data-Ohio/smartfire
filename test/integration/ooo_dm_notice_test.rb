require "test_helper"

class OooDmNoticeTest < ActionDispatch::IntegrationTest
  setup do
    @dm = rooms(:david_and_jason)
  end

  test "a DM with an OOO member shows the notice above the composer" do
    users(:david).update!(time_zone: "UTC",
      ooo_until: Time.zone.parse("2026-09-24T12:00:00Z"), ooo_note: "Back soon")
    sign_in :jason

    get room_url(@dm)

    assert_response :success
    assert_select ".ooo-notice", text: "David is out of office until September 24, 2026. Back soon"
  end

  test "the notice escapes the member's note" do
    users(:david).update!(ooo_until: 1.day.from_now, ooo_note: "<b>gone</b>")
    sign_in :jason

    get room_url(@dm)

    assert_response :success
    assert_includes response.body, "&lt;b&gt;gone&lt;/b&gt;"
    assert_not_includes response.body, "<b>gone</b>"
  end

  test "the notice renders per viewer, never from a shared fragment" do
    users(:david).update!(ooo_until: 1.day.from_now)

    sign_in :jason
    get room_url(@dm)
    assert_select ".ooo-notice", text: /David is out of office/

    sign_in :david
    get room_url(@dm)
    assert_response :success
    assert_select ".ooo-notice", count: 0
  end

  test "a group DM shows one line per OOO recipient" do
    group = Rooms::Direct.create_for({ creator: users(:kevin) },
      users: [ users(:david), users(:jason), users(:kevin) ])
    users(:david).update!(ooo_until: 1.day.from_now)
    users(:jason).update!(ooo_until: 2.days.from_now, ooo_note: "Slow to reply")
    sign_in :kevin

    get room_url(group)

    assert_response :success
    assert_select ".ooo-notice", count: 2
    assert_select ".ooo-notice", text: /David is out of office/
    assert_select ".ooo-notice", text: /Jason is out of office.*Slow to reply/
  end

  test "a channel shows no notice even while a member is out" do
    users(:david).update!(ooo_until: 1.day.from_now)
    sign_in :jason

    get room_url(rooms(:designers))

    assert_response :success
    assert_select ".ooo-notice", count: 0
  end

  test "a DM with nobody out shows no notice" do
    sign_in :jason

    get room_url(@dm)

    assert_response :success
    assert_select ".ooo-notice", count: 0
  end

  test "an OOO end broadcasts an emptied notice line" do
    users(:david).update!(ooo_until: 1.hour.from_now)
    users(:david).reload.claim_ooo_broadcast!(true)

    streams = nil
    travel_to 2.hours.from_now do
      streams = capture_turbo_stream_broadcasts([ users(:david), :ooo_notice ]) do
        Calendar::OooDispatcher.dispatch_due!
      end
    end

    assert_equal 1, streams.size
    assert_not_includes streams.first.to_html, "is out of office"
  end
end
