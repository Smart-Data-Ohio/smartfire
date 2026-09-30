# Real MessagesHelper oracle for the binary SGID payload/JSON parser/rescue contract.
require "json"
require "base64"
require "digest"
raise "unexpected JSON gem" unless Gem.loaded_specs.fetch("json").version.to_s == "2.21.2"
Rails.logger.level = :error
ActiveRecord::Base.logger = nil
Warning[:deprecated] = false
cases = JSON.parse(File.read("/tools/sgid-review-inputs.json")).map do |probe|
  [probe.fetch("name"), Base64.strict_decode64(probe.fetch("payload"))]
end
bases = [
  "", "x", "f", "false", "true", "null", "NaN", "Infinity", "-Infinity", "-", "00", "1.", "1e+", "1e9999", "-1e9999",
  "{}", "[]", '{"_rails":{}}', '{"_rails":{"data":null}}', '{"_rails":{"message":false}}', '{"_rails":1}',
  '{"a":[true,false,null,"test"],"b":12.3}', '[1,2,{"x":false},3]', '{"_rails":{"x":1,"data":"not-a-gid"}}',
  '{"_rails":{"data":"gid://campfire/User/99999"}}', '{"_rails":{"message":"AAAA"}}',
  '{"a":"\\q"}', '{"a":"\\uD800"}', '{"a":"\\uDC00"}', '{"a":"\\uD800\\uDC00"}', '{"a":"\\uXXFF"}',
  '{"a":"\\/prefix\\uD800"}', '{"a":"\\\\prefix\\uD800"}', '{"a":"\\u0000"}', '{"a":1,"a":2}',
  '//comment', '/*comment*/{}', '{/*comment*/"a":1}', '[1,//comment' + "\n2]", '{"a"/*comment*/:1}',
  "\xef\xbb\xbf{}".b, "\xff\xfe{\x00}\x00".b, "\x00\x00\xfe\xff{}".b
].map(&:b)
faults = ["\xff", "\x80", "\xc0\xaf", "\xed\xa0\x80", "\xf4\x90\x80\x80", "\xc2", "\xe2\x82", "\x00", "\n", "\r", "\t", " ", '//', '/*', '*/', '"', '\\', ',', ':', ']', '}', "\xef\xbb\xbf"].map(&:b)
# Insert faults before every byte and at EOF, including inside tokens/strings/comments.
bases.each_with_index do |base, b|
  (0..base.bytesize).each do |offset|
    faults.each_with_index do |fault, f|
      cases << ["insert #{b}/#{offset}/#{f}", base.byteslice(0, offset) + fault + base.byteslice(offset..)]
    end
  end
  (0..base.bytesize).each { |offset| cases << ["truncate #{b}/#{offset}", base.byteslice(0, offset)] }
end
# Bounded error fragments, whitespace/NUL boundaries, and invalid leading/continuation bytes.
[0, 1, 2, 30, 31, 32, 33, 64].each do |length|
  ["x", "f", "1e", '"\\q', '{"a":1,"b"', '[1,'].each_with_index do |prefix, p|
    faults.first(11).each_with_index do |fault, f|
      ["", " ", "\x00", "\n"].each_with_index do |separator, s|
        cases << ["fragment #{length}/#{p}/#{f}/#{s}", prefix.b + "a" * length + separator.b + fault + "tail"]
      end
    end
  end
end
[1, 99, 100, 101, 128, 200, 1000].each do |depth|
  ["[", '{"a":'].each_with_index do |open, o|
    close = o == 0 ? "]" : "}"
    ["0", "[]", "{}", "\xff", "/*\xff", "//\xff\n0"].each_with_index do |value, v|
      cases << ["nesting #{depth}/#{o}/#{v}", open.b * depth + value.b + close.b * depth]
    end
  end
end
random = Random.new(20260930)
Integer(ENV.fetch("SGID_RANDOM_CASES", 10000)).times do |i|
  body = bases.sample(random: random).dup
  random.rand(1..6).times do
    offset = random.rand(0..body.bytesize)
    case random.rand(5)
    when 0 then body = body.byteslice(0, offset)
    when 1 then body.slice!(offset, random.rand(1..5))
    when 2 then body.insert(offset, random.bytes(random.rand(1..5)))
    else body.insert(offset, faults.sample(random: random))
    end
  end
  cases << ["random #{i}", body]
end
request = ActionDispatch::Request.new(Rack::MockRequest.env_for("http://once.campfire.test/"))
controller = MessagesController.new
controller.set_request!(request)
controller.set_response!(ActionDispatch::Response.new)
results = Current.set(request: request) do
  ActionText::Content.with_renderer(controller) do
    cases.map do |name, payload|
      sgid = Base64.strict_encode64(payload)
      parser = begin
        JSON.parse(payload)
        { status: "valid" }
      rescue Exception => e
        { status: "error", class: e.class.name, message: Base64.strict_encode64(e.message.b),
          unloggable: !e.message.valid_encoding? }
      end
      # DB/GlobalID resolution is a crate input. Record that input independently for valid
      # JSON controls; the Rust test supplies it through the resolver, never via presentation.
      gid_lookup = nil
      if parser[:status] == "valid"
        begin
          gid = Message::MentionPreloader.gid_uri_for_sgid(sgid)
          if gid.is_a?(String)
            begin
              record = GlobalID.find(gid)
              raise "unexpected live fixture" if record
              gid_lookup = { gid: Base64.strict_encode64(gid.b), outcome: "missing" }
            rescue ActiveRecord::RecordNotFound
              gid_lookup = { gid: Base64.strict_encode64(gid.b), outcome: "missing" }
            rescue Exception => e
              gid_lookup = { gid: Base64.strict_encode64(gid.b), outcome: "raised", class: e.class.name, unloggable: !e.message.valid_encoding? }
            end
          end
        rescue Exception
          # Decoding/dig/legacy-Marshal errors occur inside the content crate before lookup.
        end
      end
      # An actual Message/ActionText body and controller view context; keep rescue logging enabled.
      message = Message.new
      message.body = %(<action-text-attachment sgid="#{sgid}"></action-text-attachment>)
      presentation = begin
        { ok: controller.view_context.message_presentation(message).to_s }
      rescue Exception => e
        { error: e.class.name }
      end
      { name: name, payload: sgid, parser: parser, presentation: presentation, gid_lookup: gid_lookup }
    end
  end
end
parser_source = File.join(Gem.loaded_specs.fetch("json").full_gem_path, "ext/json/ext/parser/parser.c")
File.write("/corpus/sgid-json.json", JSON.pretty_generate({ json_version: "2.21.2", parser_sha256: Digest::SHA256.file(parser_source).hexdigest, cases: results }) + "\n")
puts "Generated SGID helper corpus: #{results.size} cases; #{results.count { |c| c[:parser][:status] == 'error' }} JSON errors; #{results.count { |c| c[:presentation].key?(:error) }} helper raises"
