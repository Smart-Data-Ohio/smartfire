class AccountsController < ApplicationController
  before_action :ensure_can_administer, only: :update
  before_action :set_account

  def edit
    users = account_users.ordered.without_bots.includes(:google_identity, :two_factor_credential)
    @administrators, @members = users.partition(&:administrator?)
    set_page_and_extract_portion_from users, per_page: 500
  end

  def update
    previous_name = @account.name
    previous_restrict = @account.settings.restrict_room_creation_to_administrators?
    previous_logo = @account.logo.attached?
    @account.update!(account_params)
    record_settings_changes(previous_name: previous_name, previous_restrict: previous_restrict, previous_logo: previous_logo)
    redirect_to edit_account_url, notice: "✓"
  end

  private
    def set_account
      @account = Current.account
    end

    def account_params
      params.require(:account).permit(:name, :logo, settings: {})
    end

    def account_users
      if Current.user.can_administer?
        User.where(status: [ :active, :banned ])
      else
        User.active
      end
    end

    def record_settings_changes(previous_name:, previous_restrict:, previous_logo:)
      changes = {}
      changes[:name] = AuditLog.pair(previous_name, @account.name) if @account.name != previous_name
      current_restrict = @account.settings.restrict_room_creation_to_administrators?
      if current_restrict != previous_restrict
        changes[:restrict_room_creation_to_administrators] = AuditLog.pair(previous_restrict, current_restrict)
      end
      changes[:logo] = AuditLog.pair(previous_logo, @account.logo.attached?) if @account.logo.attached? != previous_logo

      AuditLog.record!(action: "account.settings.change", target: @account, changes: changes) if changes.present?
    end
end
