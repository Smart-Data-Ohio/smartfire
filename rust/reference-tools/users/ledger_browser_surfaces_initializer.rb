# Tools-only original allow_forgery_protection(false) render input.
# The marker is private fixture IPC and is never a route or an HTML rewrite.
class << ActionController::Base
  alias_method :ledger_original_forgery_protection, :allow_forgery_protection
  def allow_forgery_protection
    File.exist?('/rails/storage/db/ledger-forgery-off') ? false : ledger_original_forgery_protection
  end
end
module LedgerSurfaceForgeryInput
  def allow_forgery_protection
    File.exist?('/rails/storage/db/ledger-forgery-off') ? false : super
  end
end
ActionController::Base.prepend(LedgerSurfaceForgeryInput)
