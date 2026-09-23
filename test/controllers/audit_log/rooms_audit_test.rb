require "test_helper"

class AuditLog::RoomsAuditTest < ActionDispatch::IntegrationTest
  setup do
    sign_in :david
  end

  test "room create is recorded" do
    assert_difference -> { AuditLog.where(action: "room.create").count }, +1 do
      post rooms_opens_url, params: { room: { name: "Audited Room" } }
    end

    entry = AuditLog.where(action: "room.create").last
    assert_equal users(:david).id, entry.actor_id
    assert_equal Rooms::Open.last.id, entry.target_id
    assert_equal "Room", entry.target_type
    assert_equal "Audited Room", entry.target_label
    assert_equal({ "name" => "Audited Room" }, entry.details)
  end

  test "failed room create writes no row" do
    assert_no_difference -> { AuditLog.where(action: "room.create").count } do
      post rooms_closeds_url, params: { room: { name: "Iconic", icon_name: ":notanicon:" }, user_ids: [ users(:david).id ] }
    end

    assert_response :unprocessable_entity
  end

  test "opening an existing direct room writes no row" do
    jason, kevin = users(:jason), users(:kevin)

    assert_difference -> { AuditLog.where(action: "room.create").count }, +1 do
      post rooms_directs_url, params: { user_ids: [ users(:david).id, jason.id, kevin.id ] }
    end

    assert_no_difference -> { AuditLog.where(action: "room.create").count } do
      post rooms_directs_url, params: { user_ids: [ users(:david).id, jason.id, kevin.id ] }
    end
  end

  test "room destroy is recorded with the room name" do
    room = Rooms::Closed.create_for({ name: "Doomed", creator: users(:david) }, users: [ users(:david) ])

    assert_difference -> { AuditLog.where(action: "room.destroy").count }, +1 do
      delete room_url(room)
    end

    entry = AuditLog.where(action: "room.destroy").last
    assert_equal users(:david).id, entry.actor_id
    assert_equal room.id, entry.target_id
    assert_equal "Doomed", entry.target_label
  end

  test "admin membership changes are recorded with names" do
    room = rooms(:designers)
    add = users(:kevin)
    room.memberships.where(user: add).delete_all
    remove = room.users.where.not(id: users(:david).id).first
    kept_ids = room.user_ids - [ remove.id ]

    assert_difference -> { AuditLog.where(action: "room.membership.change").count }, +1 do
      put rooms_closed_url(room), params: {
        room: { name: room.name }, user_ids: kept_ids + [ add.id ]
      }
    end

    entry = AuditLog.where(action: "room.membership.change").last
    assert_equal room.id, entry.target_id
    assert_equal [ add.name ], entry.details["granted"]
    assert_equal [ remove.name ], entry.details["revoked"]
  end

  test "re-saving unchanged membership writes no row" do
    room = rooms(:designers)

    assert_no_difference -> { AuditLog.where(action: "room.membership.change").count } do
      put rooms_closed_url(room), params: { room: { name: room.name }, user_ids: room.user_ids }
    end
  end

  test "adding group members is recorded with the added users" do
    room = create_group_dm!([ users(:david), users(:jason), users(:kevin) ])

    assert_difference -> { AuditLog.where(action: "room.membership.change").count }, +1 do
      post add_members_rooms_direct_url(room), params: { user_ids: [ users(:jz).id ] }
    end

    entry = AuditLog.where(action: "room.membership.change").last
    assert_equal users(:david).id, entry.actor_id
    assert_equal room.id, entry.target_id
    assert_equal [ "JZ" ], entry.details["granted"]
  end

  test "adding no new members writes no row" do
    room = create_group_dm!([ users(:david), users(:jason), users(:kevin) ])

    assert_no_difference -> { AuditLog.where(action: "room.membership.change").count } do
      post add_members_rooms_direct_url(room), params: { user_ids: [ users(:jason).id ] }
    end
  end

  test "the last member out destroys the group and records it" do
    room = create_group_dm!([ users(:david), users(:jason), users(:kevin) ])
    room.rename("Weekend Plans", renamed_by: users(:david))
    delete leave_rooms_direct_url(room)
    sign_in :jason
    delete leave_rooms_direct_url(room)

    sign_in :kevin
    assert_difference -> { AuditLog.where(action: "room.destroy").count }, +1 do
      delete leave_rooms_direct_url(room)
    end

    entry = AuditLog.where(action: "room.destroy").last
    assert_equal users(:kevin).id, entry.actor_id
    assert_equal room.id, entry.target_id
    assert_equal "Weekend Plans", entry.target_label
  end

  test "leaving without destroying the group writes no row" do
    room = create_group_dm!([ users(:david), users(:jason), users(:kevin) ])

    assert_no_difference -> { AuditLog.where(action: "room.destroy").count } do
      delete leave_rooms_direct_url(room)
    end
  end

  test "account settings changes are recorded" do
    assert_difference -> { AuditLog.where(action: "account.settings.change").count }, +1 do
      patch account_url, params: {
        account: {
          name: "Renamed Account",
          settings: { restrict_room_creation_to_administrators: !Current.account.settings.restrict_room_creation_to_administrators? }
        }
      }
    end

    entry = AuditLog.where(action: "account.settings.change").last
    assert_equal Current.account.id, entry.target_id
    assert_equal "Account", entry.target_type
    assert entry.details.key?("name")
    assert entry.details.key?("restrict_room_creation_to_administrators")
  end

  test "logo removal is recorded" do
    accounts(:signal).update! logo: fixture_file_upload("moon.jpg", "image/jpeg")

    assert_difference -> { AuditLog.where(action: "account.settings.change").count }, +1 do
      delete account_logo_url
    end

    assert_equal [ true, false ], AuditLog.where(action: "account.settings.change").last.details["logo"]
  end

  test "custom styles changes are recorded" do
    previous = Current.account.custom_styles

    assert_difference -> { AuditLog.where(action: "account.custom_styles.change").count }, +1 do
      put account_custom_styles_url, params: { account: { custom_styles: "body { color: red; }" } }
    end

    entry = AuditLog.where(action: "account.custom_styles.change").last
    assert_equal [ previous, "body { color: red; }" ], entry.details["custom_styles"]
  end

  test "unchanged custom styles write no row" do
    Current.account.update!(custom_styles: "body { color: red; }")

    assert_no_difference -> { AuditLog.where(action: "account.custom_styles.change").count } do
      put account_custom_styles_url, params: { account: { custom_styles: "body { color: red; }" } }
    end
  end

  test "workspace icon create and destroy are recorded" do
    assert_difference -> { AuditLog.where(action: "workspace_icon.create").count }, +1 do
      post account_icons_url, params: {
        workspace_icon: {
          name: "acme", title: "Acme Corp",
          image: fixture_file_upload("workspace_icons/clean.svg", "image/svg+xml")
        }
      }
    end

    icon = WorkspaceIcon.find_by!(name: "acme")
    create = AuditLog.where(action: "workspace_icon.create").last
    assert_equal icon.id, create.target_id
    assert_equal "WorkspaceIcon", create.target_type
    assert_equal ":acme:", create.target_label

    assert_difference -> { AuditLog.where(action: "workspace_icon.destroy").count }, +1 do
      delete account_icon_url(icon)
    end

    destroy = AuditLog.where(action: "workspace_icon.destroy").last
    assert_equal icon.id, destroy.target_id
    assert_equal ":acme:", destroy.target_label
  end

  private
    def create_group_dm!(*members)
      members = members.flatten
      Current.set(user: members.first) { Rooms::Direct.find_or_create_for(members) }
    end
end
