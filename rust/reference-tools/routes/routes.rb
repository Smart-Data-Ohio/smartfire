# The reference's route table and named routes, for crates/routes (`campfire_routes`), whose
# build.rs turns this file into the route table the app dispatches on and the `*_path` helpers:
#
#   parity/bin/reference runner reference-tools/routes/routes.rb > crates/routes/routes.json
#
# `table` is every route in `bin/rails routes` order, with whether its controller and action
# exist: Rails answers a declared route with no action method and no template with
# AbstractController::ActionNotFound (404), and one whose controller doesn't load with a 500.
#
# Each helper is called with every required part set to plain, awkward (reserved and non-ASCII
# characters) and numeric values, with a format, and with extra query parameters, so the Rust
# helpers' segment escaping, format suffix and query string are checked against Journey's.
require "json"

routes = Rails.application.routes
helpers = routes.url_helpers
AWKWARD = "a b/c?d#e%f&g=h+i.j~k:l@m!n$o'p(q)r*s,t;u=v é"

named = routes.named_routes.names.sort_by(&:to_s).map do |name|
  route = routes.named_routes.get(name)
  # Minimal positional samples omit defaults; callers may also override those parts positionally.
  required = route.required_parts - route.defaults.keys
  helper = "#{name}_path"
  call = ->(args, options = {}) { helpers.public_send(helper, *args, **options) }

  samples = []
  samples << { args: required.map { "1" }, options: {} }
  samples << { args: required.each_with_index.map { |_, i| "#{AWKWARD}#{i}" }, options: {} } if required.any?
  samples << { args: required.map { "7" }, options: { format: "json" } } if route.path.spec.to_s.include?("(.:format)")
  samples << { args: required.map { "2" }, options: { "q" => "x y&z", "a" => "1", "empty" => "", "gone" => nil } }
  route.defaults.except(:controller, :action).each_key do |key|
    # Defaults are overridable keyword options, not immutable path literals.
    next unless route.parts.include?(key)
    [ "7", AWKWARD ].each do |value|
      samples << { args: required.map { "2" }, options: { key => value, q: "kept" }, default_override: key.to_s }
    end
  end
  samples.each { |sample| sample[:path] = call.(sample[:args], sample[:options].transform_keys(&:to_sym)) }


  { name: name.to_s, verb: route.verb, spec: route.path.spec.to_s, endpoint: "#{route.defaults[:controller]}##{route.defaults[:action]}",
    arguments: required.map(&:to_s), optional_parts: (route.parts - route.required_parts).map(&:to_s),
    defaults: route.defaults.except(:controller, :action).transform_values(&:to_s), samples: samples }
end

# `direct` routes are Ruby blocks, not Journey routes; record what they build for fixed inputs.
# They read only these attributes, so plain structs stand in for the records.
# `route_for` inside them builds a URL, which needs a host even for the _path variant.
routes.default_url_options[:host] = "campfire.test"
stamp = Time.utc(2026, 3, 2, 16, 0, 0)
account = Struct.new(:updated_at).new(stamp)
Account.define_singleton_method(:first) { account } # Current.account is Account.first
directs = {
  fresh_account_logo: [ helpers.fresh_account_logo_path, helpers.fresh_account_logo_path(size: :small) ],
  fresh_user_avatar: [ helpers.fresh_user_avatar_path(Struct.new(:avatar_token, :updated_at).new("tok/en+x", stamp)) ]
}
account = nil
directs[:fresh_account_logo_without_account] = [ helpers.fresh_account_logo_path, helpers.fresh_account_logo_path(size: :large) ]

def action_status(controller_path, action)
  controller = "#{controller_path.camelize}Controller".safe_constantize
  return "missing_controller" unless controller.is_a?(Class) && controller < AbstractController::Base
  return "defined" if controller.action_methods.include?(action)
  # ActionController::ImplicitRender: an action with a template but no method still renders.
  if controller.respond_to?(:_prefixes) && controller.new.lookup_context.exists?(action, controller._prefixes)
    "implicit"
  else
    "action_not_found"
  end
end

table = routes.routes.filter_map do |route|
  next if route.verb.blank? || route.internal
  controller, action = route.defaults.values_at(:controller, :action)
  { verb: route.verb, spec: route.path.spec.to_s, endpoint: "#{controller}##{action}", name: route.name,
    defaults: route.defaults.except(:controller, :action).transform_values(&:to_s),
    action: action_status(controller.to_s, action.to_s) }
end

puts JSON.pretty_generate(table: table, named_routes: named, directs: directs)
