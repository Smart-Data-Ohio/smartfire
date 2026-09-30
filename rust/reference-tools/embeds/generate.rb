# Generate WS15e contracts from the pinned Smartfire Rails image, without outbound traffic.
require "restricted_http/private_network_guard"
require "socket"

output = ARGV.fetch(0)
source = File.read(Rails.root.join("test/lib/restricted_http/private_network_guard_test.rb"))
addresses = source.scan(/(?:assert_private_ip|private_ip\?)\s*(?:\(\s*)?"([^"]*)"/).flatten
addresses += %w[168.63.129.16 192.0.0.9 2001:3::1 2001:4:112::1 4000::1 3fff::1 5f00::1]
addresses.uniq!
hosts = %w[private.example.com example.com images.example.com nxdomain.example.com
  2130706433 0x7f000001 017700000001 127.1 3232235521 134744072 8.8.8.8 [::1]
  [8.8.8.8] [2606:4700:4700::1111] 0x08080808 010.010.010.010 8.8.2056
  under_score.example -lead.example example.com. 09.1.1.1 1.2.3.4.]
hosts += [ "", "a..b", "host%eth0", "exämple.com", "[v1.x]" ]
host_cases = hosts.map { |host| { "host" => host, "answers" => ["93.184.216.34"] } }
host_cases += %w[::1 fd00::1 fe80::1 ::ffff:127.0.0.1].map { |ip| { "host" => "images.example.com", "answers" => [ip] } }
host_cases += [
  { "host" => "private.example.com", "answers" => ["192.168.1.1"] },
  { "host" => "nxdomain.example.com", "answers" => [] },
  { "host" => "mixed.example", "answers" => ["::1", "2606:4700:4700::1111", "10.0.0.1", "93.184.216.34"] }
]
$guard_answers = []
$guard_lookups = []
Resolv.singleton_class.prepend(Module.new do
  define_method(:getaddresses) do |host|
    $guard_lookups << host
    $guard_answers
  end
end)
host_cases.each do |entry|
  $guard_answers = entry["answers"]
  $guard_lookups = []
  entry["result"] = begin
    { "ip" => RestrictedHTTP::PrivateNetworkGuard.resolve(entry["host"]) }
  rescue RestrictedHTTP::Violation
    { "error" => "violation" }
  rescue Surfguard::Unresolvable
    { "error" => "unresolvable" }
  end
  entry["lookups"] = $guard_lookups
end

bases = ["https://example.com/a/b?old=1#old", "http://example.com", "https://example.com/a/../b/c"]
locations = [nil, "", " ", "/other", "../next", "./", "../../../../next", "?new=2", "#frag", "//cdn.example.com/x", "//cdn.example.com:51550/x", "/a//b/../c", "https://cdn.example.com/a/../x", "http://bad host/", "javascript:alert(1)", "/%2e%2e/x", "/bad%zz", "?q=hello world"]
redirects = bases.product(locations).map do |base, location|
  result = begin
    raise Opengraph::Fetch::RedirectDeniedError if location.blank?
    uri = URI.parse(base).merge(location.to_s)
    raise Opengraph::Fetch::RedirectDeniedError unless uri.is_a?(URI::HTTP)
    { "url" => uri.to_s }
  rescue URI::InvalidURIError, Opengraph::Fetch::RedirectDeniedError
    { "error" => "denied" }
  end
  { "base" => base, "location" => location, "result" => result }
end
urls = ["https://images.example.com/photo.png", "http://example.com/a?b=1&c=2", "https://example.com/%20.png", "javascript:alert(1)", "//images.example.com/x.png"]
signed = urls.map do |url|
  token = Embeds::ImageProxy.verifier.generate(url)
  { "url" => url, "signed" => token, "path" => Embeds::ImageProxy.signed_path(url), "verified_url" => Embeds::ImageProxy.verified_url(token) }
end
attachment = %(<action-text-attachment content-type="application/vnd.actiontext.opengraph-embed" href="https://example.com/page" url="https://example.com/image.png" filename="Title" caption="Description"></action-text-attachment>)
node = ActionText::Fragment.wrap(attachment).find_all(ActionText::Attachment.tag_name).first
embed = ActionText::Attachment.from_node(node)
embed_html = ApplicationController.renderer.render(partial: "action_text/attachables/opengraph_embed", locals: { opengraph_embed: embed })

# The same response construction as ImagesController#show, with Rails' actual header writer.
controller = Class.new(ActionController::Base) do
  def show
    expires_in 1.hour, private: true
    send_data "PNG-BYTES", type: "image/png", disposition: "inline"
  end
end
status, headers, body = controller.action(:show).call(Rack::MockRequest.env_for("/embeds/image/fixture"))
File.write(output, JSON.pretty_generate({
  "reference" => "d7c7de92", "addresses" => addresses.map { |ip| { "address" => ip, "blocked" => RestrictedHTTP::PrivateNetworkGuard.private_ip?(ip) } },
  "hosts" => host_cases, "redirects" => redirects, "signed" => signed,
  "embed_html" => embed_html, "image_response" => { "status" => status, "cache_control" => headers["cache-control"], "content_type" => headers["content-type"], "content_disposition" => headers["content-disposition"], "content_transfer_encoding" => headers["content-transfer-encoding"] }
}) + "\n")
puts "WS15e Rails vectors: #{addresses.size} addresses, #{host_cases.size} hosts, #{redirects.size} redirects, #{signed.size} signatures"
