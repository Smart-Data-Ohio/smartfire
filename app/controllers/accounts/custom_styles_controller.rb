class Accounts::CustomStylesController < ApplicationController
  before_action :ensure_can_administer, :set_account
  before_action :require_sudo_mode, only: :update

  def edit
  end

  def update
    previous_styles = @account.custom_styles
    @account.update!(account_params)
    if @account.custom_styles != previous_styles
      AuditLog.record!(action: "account.custom_styles.change", target: @account,
        changes: { custom_styles: AuditLog.pair(style_summary(previous_styles), style_summary(@account.custom_styles)) })
    end
    redirect_to edit_account_custom_styles_url, notice: "✓"
  end

  private
    # The log keeps a size and digest of each revision, not the full
    # stylesheet: enough to see that it changed and which revision is
    # current, without storing arbitrary CSS for a year.
    def style_summary(styles)
      { size: styles.to_s.bytesize, digest: Digest::SHA256.hexdigest(styles.to_s)[0, AuditLog::DIGEST_PREFIX_LENGTH] }
    end

    def set_account
      @account = Current.account
    end

    def account_params
      params.require(:account).permit(:custom_styles)
    end
end
