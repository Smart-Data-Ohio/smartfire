# The sudo prompt: confirms the signed-in member's identity before a
# sensitive action continues. Password by default, a TOTP code as an
# alternative for members with two-step sign-in enrolled, and Google
# re-auth for members with a linked Google identity (Google-only
# members have no password).
class SudosController < ApplicationController
  include GoogleSignInFlow

  # The default shared-cache store (like SessionsController), so the limit
  # holds across Puma workers. Tests swap in a memory store, since the
  # test cache store is null.
  rate_limit to: 10, within: 3.minutes, only: %i[ create google ], with: -> { render_sudo_rejection }

  def new
    @verifiers = sudo_verifiers_for(Current.user)
  end

  def create
    verifier = params[:verifier].to_s.presence || "password"

    case SudoMode.verify_with(verifier, Current.user, params)
    when true
      mark_sudo_verified!
      AuditLog.record!(action: "sudo.confirm.success", changes: { verifier: verifier })
      continue_after_sudo!
    when false
      AuditLog.record!(action: "sudo.confirm.failure", changes: { verifier: verifier })
      flash.now[:alert] = "Confirmation failed. Try again."
      @verifiers = sudo_verifiers_for(Current.user)
      render :new, status: :unauthorized
    else
      flash.now[:alert] = "That confirmation method is not available."
      @verifiers = sudo_verifiers_for(Current.user)
      render :new, status: :unprocessable_entity
    end
  end

  # Starts a Google re-auth: the member proves the linked Google account
  # is theirs, and Sessions::GoogleController#callback finishes the
  # confirmation (purpose "sudo") instead of signing anyone in.
  def google
    unless Google::SignIn.configured? && Current.user.google_identity
      return redirect_to new_sudo_url, alert: "Google confirmation is not available for your account."
    end

    redirect_to_google_sign_in(purpose: "sudo", user_id: Current.user.id)
  end

  private
    def render_sudo_rejection
      flash.now[:alert] = "Too many confirmation attempts. Try again in a few minutes."
      @verifiers = sudo_verifiers_for(Current.user)
      render :new, status: :too_many_requests
    end
end
