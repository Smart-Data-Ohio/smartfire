require "socket"
require "surfguard"

module RestrictedHTTP
  class Violation < StandardError; end

  # The address policy lives in the surfguard gem so this app, Basecamp, HEY and
  # Fizzy classify "internal" the same way instead of each keeping a copy that
  # drifts. The callers here hand a bare hostname in and pin the address that
  # comes back, so this stays a hostname-in, address-out shim.
  module PrivateNetworkGuard
    extend self

    # A hostname that resolves to nothing (NXDOMAIN, timeout, empty answer)
    # raises Surfguard::Unresolvable, which we let propagate as a lookup failure
    # -- the same way the old Resolv.getaddress guard raised Resolv::ResolvError,
    # and the callers here already treat it as a fetch failure. The Violation is
    # reserved for a host that resolves to a blocked address (an empty list back
    # from resolve_public_ips), so a transient DNS miss is never misreported as
    # an SSRF attempt.
    def resolve(hostname)
      if numeric = numeric_ip_for(hostname)
        raise Violation.new("Attempt to access private IP via #{hostname}") if Surfguard.blocked_address?(numeric)
        return numeric
      end

      Surfguard.resolve_public_ips(hostname).first or
        raise Violation.new("Attempt to access private IP via #{hostname}")
    end

    def private_ip?(ip)
      Surfguard.blocked_address?(ip)
    end

    private
      # A numeric host in any form the connection layer decodes (dotted
      # quads, decimal, octal, and hex integers, short dotted forms),
      # normalized to the address getaddrinfo would connect to. Resolved
      # without DNS (AI_NUMERICHOST), so a numeric host never falls
      # through to a DNS lookup that answers a different address. Nil
      # unless the whole host parses as an IP.
      def numeric_ip_for(host)
        bare = host.to_s.delete_prefix("[").delete_suffix("]")
        Addrinfo.getaddrinfo(bare, nil, nil, :STREAM, nil, Socket::AI_NUMERICHOST).first&.ip_address
      rescue SocketError
        nil
      end
  end
end
