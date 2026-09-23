require "test_helper"

class Rooms::InboundEmailAddressesControllerTest < ActionDispatch::IntegrationTest
  setup do
    @room = rooms(:designers)
  end

  test "an administrator can create the room address" do
    sign_in :jason

    post room_inbound_email_address_url(@room)

    assert_redirected_to edit_room_path(@room)
    assert @room.reload.inbound_email_token.present?
  end

  test "the creator can rotate the address" do
    sign_in :david
    old_token = @room.regenerate_inbound_email_token!

    post room_inbound_email_address_url(@room)

    assert_redirected_to edit_room_path(@room)
    assert_not_equal old_token, @room.reload.inbound_email_token
  end

  test "a plain member is forbidden" do
    sign_in :jz

    post room_inbound_email_address_url(@room)

    assert_response :forbidden
    assert_nil @room.reload.inbound_email_token
  end

  test "a non-member gets a 404" do
    memberships(:david_designers).destroy!
    sign_in :david

    post room_inbound_email_address_url(@room)

    assert_response :not_found
  end

  test "direct rooms never have an address" do
    sign_in :david

    post room_inbound_email_address_url(rooms(:david_and_jason))

    assert_response :not_found
  end

  test "board rooms never have an address" do
    board = Rooms::Board.create_for({ name: "Launch", creator: users(:david) }, users: [ users(:david) ])
    sign_in :david

    post room_inbound_email_address_url(board)

    assert_response :not_found
    assert_nil board.reload.inbound_email_token
  end

  test "the edit page shows the address once created" do
    ENV["INBOUND_EMAIL_DOMAIN"] = "mail.test"
    sign_in :david
    @room.regenerate_inbound_email_token!

    get edit_rooms_closed_url(@room)

    assert_response :success
    assert_includes response.body, @room.reload.inbound_email_address
  ensure
    ENV.delete("INBOUND_EMAIL_DOMAIN")
  end

  test "the edit page explains the missing domain" do
    ENV.delete("INBOUND_EMAIL_DOMAIN")
    sign_in :david

    get edit_rooms_closed_url(@room)

    assert_response :success
    assert_includes response.body, "INBOUND_EMAIL_DOMAIN"
  end
end
