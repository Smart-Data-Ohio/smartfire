class AddLockoutToTwoFactorCredentials < ActiveRecord::Migration[8.2]
  # Exponential backoff for the two-factor challenge: after a run of
  # failed codes the challenge locks for an escalating duration (see
  # TwoFactorCredential::LOCKOUT_DURATIONS). consecutive_failures counts
  # the current run (reset on success and whenever a lockout starts);
  # lockout_count escalates the duration across lockouts without a
  # success in between.
  def change
    add_column :two_factor_credentials, :consecutive_failures, :integer, null: false, default: 0
    add_column :two_factor_credentials, :locked_until, :datetime
    add_column :two_factor_credentials, :lockout_count, :integer, null: false, default: 0
  end
end
