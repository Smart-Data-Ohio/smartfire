require "test_helper"

class Rooms::Boards::AutomationsControllerTest < ActionDispatch::IntegrationTest
  setup do
    @board = Rooms::Board.create_for({ name: "Launch", creator: users(:david) },
      users: [ users(:david), users(:jz), users(:kevin) ])
  end

  test "the board creator sees the automations page" do
    sign_in :david

    get board_automations_path(@board)

    assert_response :success
    assert_match "Auto-assign by tag", response.body
    assert_match "SLA timers", response.body
  end

  test "an administrator sees the automations page" do
    users(:kevin).update!(role: "administrator")
    sign_in :kevin

    get board_automations_path(@board)

    assert_response :success
  end

  test "a plain member is forbidden" do
    sign_in :jz

    get board_automations_path(@board)
    assert_response :forbidden

    post board_automation_tag_assignments_path(@board), params: { tag: "bug", assignee_id: users(:jz).id }
    assert_response :forbidden

    patch board_automation_sla_rules_path(@board), params: { sla_rules: { "planned" => { nudge_after_minutes: "30", escalate_after_minutes: "60" } } }
    assert_response :forbidden
  end

  test "a non-member is redirected" do
    sign_in :jason

    get board_automations_path(@board)

    assert_redirected_to root_url
  end

  test "creating a tag rule audits the change" do
    sign_in :david

    assert_difference -> { @board.board_tag_assignments.count }, 1 do
      assert_difference -> { AuditLog.where(action: "board.automation.change").count }, 1 do
        post board_automation_tag_assignments_path(@board), params: { tag: "bug", assignee_id: users(:jz).id }
      end
    end

    assert_redirected_to board_automations_path(@board)
    row = AuditLog.where(action: "board.automation.change").order(:id).last
    assert_equal "created", row.details["tag_rule"]
    assert_equal "bug", row.details["tag"]
  end

  test "creating a tag rule rejects an invalid tag" do
    sign_in :david

    assert_no_difference -> { @board.board_tag_assignments.count } do
      post board_automation_tag_assignments_path(@board), params: { tag: "Not A Tag!", assignee_id: users(:jz).id }
    end

    assert_response :unprocessable_entity
  end

  test "creating a tag rule rejects an ineligible assignee" do
    sign_in :david

    assert_no_difference -> { @board.board_tag_assignments.count } do
      post board_automation_tag_assignments_path(@board), params: { tag: "bug", assignee_id: users(:jason).id }
    end

    assert_response :unprocessable_entity
  end

  test "removing a tag rule audits the change" do
    rule = BoardTagAssignment.create!(room: @board, tag: "bug", assignee: users(:jz), created_by: users(:david))
    sign_in :david

    assert_difference -> { @board.board_tag_assignments.count }, -1 do
      delete board_automation_tag_assignment_path(@board, rule)
    end

    assert_redirected_to board_automations_path(@board)
    row = AuditLog.where(action: "board.automation.change").order(:id).last
    assert_equal "removed", row.details["tag_rule"]
    assert_equal "bug", row.details["tag"]
  end

  test "removing an unknown rule redirects with an alert" do
    sign_in :david

    delete board_automation_tag_assignment_path(@board, 123_456)

    assert_redirected_to board_automations_path(@board)
  end

  test "saving sla timers upserts rules and audits each change" do
    sign_in :david

    assert_difference -> { @board.board_sla_rules.count }, 2 do
      patch board_automation_sla_rules_path(@board), params: {
        sla_rules: {
          "planned" => { nudge_after_minutes: "60", escalate_after_minutes: "240" },
          "blocked" => { nudge_after_minutes: "30", escalate_after_minutes: "120" },
          "in_progress" => { nudge_after_minutes: "", escalate_after_minutes: "" },
          "done" => { nudge_after_minutes: "", escalate_after_minutes: "" }
        }
      }
    end

    assert_redirected_to board_automations_path(@board)
    assert_equal 60, @board.board_sla_rules.find_by(work_status: "planned").nudge_after_minutes
    assert_equal 2, AuditLog.where(action: "board.automation.change").count
  end

  test "saving sla timers with an invalid threshold saves nothing" do
    sign_in :david

    assert_no_difference -> { @board.board_sla_rules.count } do
      patch board_automation_sla_rules_path(@board), params: {
        sla_rules: {
          "planned" => { nudge_after_minutes: "60", escalate_after_minutes: "30" }
        }
      }
    end

    assert_response :unprocessable_entity
    assert_match "must be after the nudge threshold", response.body
  end

  test "blanking a status removes its rule" do
    @board.board_sla_rules.create!(work_status: "planned", nudge_after_minutes: 60, escalate_after_minutes: 240)
    sign_in :david

    assert_difference -> { @board.board_sla_rules.count }, -1 do
      patch board_automation_sla_rules_path(@board), params: {
        sla_rules: { "planned" => { nudge_after_minutes: "", escalate_after_minutes: "" } }
      }
    end

    row = AuditLog.where(action: "board.automation.change").order(:id).last
    assert_equal "removed", row.details["sla_rule"]
  end

  test "re-saving unchanged timers writes no audit row" do
    @board.board_sla_rules.create!(work_status: "planned", nudge_after_minutes: 60, escalate_after_minutes: 240)
    sign_in :david

    assert_no_difference -> { AuditLog.where(action: "board.automation.change").count } do
      patch board_automation_sla_rules_path(@board), params: {
        sla_rules: { "planned" => { nudge_after_minutes: "60", escalate_after_minutes: "240" } }
      }
    end

    assert_redirected_to board_automations_path(@board)
  end
end
