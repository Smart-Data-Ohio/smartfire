class Accounts::CustomStylesController < ApplicationController
  before_action :ensure_can_administer, :set_account

  def edit
  end

  def update
    previous_styles = @account.custom_styles
    @account.update!(account_params)
    if @account.custom_styles != previous_styles
      AuditLog.record!(action: "account.custom_styles.change", target: @account,
        changes: { custom_styles: AuditLog.pair(previous_styles, @account.custom_styles) })
    end
    redirect_to edit_account_custom_styles_url, notice: "✓"
  end

  private
    def set_account
      @account = Current.account
    end

    def account_params
      params.require(:account).permit(:custom_styles)
    end
end
