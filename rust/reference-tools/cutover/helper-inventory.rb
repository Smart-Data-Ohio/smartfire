#!/usr/bin/env ruby
# Parse source bytes supplied by helper_inventory.py; never load the Rails app.
require "json"
require "ripper"

def constant(node)
  return unless node.is_a?(Array)
  case node[0]
  when :@const then node[1]
  when :const_ref, :var_ref then constant(node[1])
  when :top_const_ref then constant(node[1])
  when :const_path_ref, :const_path_field
    [constant(node[1]), constant(node[2])].compact.join("::")
  end
end

def constants(node)
  return [] unless node.is_a?(Array)
  name = constant(node)
  return [name] if name
  node.flat_map { |child| constants(child) }
end

def call(node)
  return unless node.is_a?(Array)
  case node[0]
  when :command, :fcall, :vcall
    [node[1], "self"]
  when :call, :command_call
    target = node[1]
    receiver = target.is_a?(Array) && target[0] == :var_ref && target[1][0..1] == [:@kw, "self"] ? "self" : "other"
    [node[3], receiver]
  when :method_add_arg, :method_add_block then call(node[1])
  end
end

def calls(node, result = [])
  return result unless node.is_a?(Array)
  # Wrappers contain the same call node, so record only actual call forms.
  if [:command, :fcall, :vcall, :call, :command_call].include?(node[0])
    token, receiver = call(node)
    result << { "name" => token[1], "line" => token[2][0], "receiver" => receiver }
  end
  node.each { |child| calls(child, result) if child.is_a?(Array) }
  result
end

def body(node)
  found = calls(node).uniq
  { "calls" => found, "assertions" => found.select { |c| c["name"].match?(/\A(?:assert|refute)(?:_\w+)?\z/) || c["name"] == "expects" }.map { |c| c["line"] }.uniq.sort }
end

def scan(node, owner, namespace, file, scopes, declarations)
  return unless node.is_a?(Array)
  case node[0]
  when :class, :module
    name = constant(node[1])
    name = "#{namespace}::#{name}" if !namespace.empty? && !name.include?("::")
    scope = (scopes[name] ||= { "includes" => [], "methods" => {}, "callbacks" => [] })
    scope["parent"] = constant(node[2]) if node[0] == :class && node[2]
    scan(node[node[0] == :class ? 3 : 2], name, name, file, scopes, declarations)
    return
  when :def
    token = node[1]
    scopes.fetch(owner)["methods"][token[1]] = body(node[2..]).merge("file" => file, "line" => token[2][0])
    return
  when :method_add_block
    token, = call(node[1])
    if token && token[1] == "test"
      declarations["#{file}:#{token[2][0]}"] = body(node[2]).merge("owner" => owner)
      return
    elsif token && ["setup", "teardown"].include?(token[1])
      scopes.fetch(owner)["callbacks"] << body(node[2]).merge("file" => file, "helper" => token[1])
      return
    end
  when :command, :method_add_arg
    token, = call(node)
    if token && ["include", "prepend"].include?(token[1])
      scopes.fetch(owner)["includes"].concat(constants(node[2]))
      return
    elsif token && ["setup", "teardown"].include?(token[1])
      # Rails also permits setup :method_name (and the equivalent teardown).
      symbols = []
      collect_symbols = lambda do |child|
        next unless child.is_a?(Array)
        if child[0] == :symbol && child[1].is_a?(Array)
          symbols << { "name" => child[1][1], "line" => child[1][2][0], "receiver" => "self" }
        else
          child.each { |part| collect_symbols.call(part) }
        end
      end
      collect_symbols.call(node)
      scopes.fetch(owner)["callbacks"] << { "file" => file, "helper" => token[1], "calls" => symbols, "assertions" => [] }
      return
    end
  end
  node.each { |child| scan(child, owner, namespace, file, scopes, declarations) if child.is_a?(Array) }
end

sources = JSON.parse($stdin.read)
scopes, declarations = {}, {}
sources.each do |file, source|
  ast = Ripper.sexp(source)
  abort "Unable to parse pinned Ruby source #{file}" unless ast
  owner = "<#{file}>"
  scopes[owner] = { "includes" => [], "methods" => {}, "callbacks" => [] }
  scan(ast, owner, "", file, scopes, declarations)
end
puts JSON.generate({ "scopes" => scopes, "declarations" => declarations })
