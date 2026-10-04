require "json"
require "digest"
JSON.parse(File.read(ENV.fetch("GITHUB_REFERENCE_HASHES"))).each do |path, hash|
  raise "Reference drift: #{path}" unless Digest::SHA256.file(Rails.root.join(path)).hexdigest == hash
end
texts = [nil, "", "plain text", "https://github.com/rails/rails/pull/123",
  "see https://github.com/o/r/pulls/45/files?diff=split#diff-1 please",
  "https://github.com/o/r/pull/0007 https://github.com/o/r/pull/7",
  "https://github.com/O/R/pull/1 https://github.com/o/r/pull/1",
  (1..6).map { |n| "https://github.com/o/r/pull/#{n}" }.join(" "),
  "https://github.com/o/r/pull/0", "https://github.com/foo.bar/baz-qux/pull/7"]
%w[issues pulls pull PULL].each { |part| texts << "https://github.com/o/r/#{part}/123" }
%w[http://github.com https://example.com https://GITHUB.com].each { |host| texts << "#{host}/o/r/pull/123" }
%w[. .. ... foo.bar _ -].each do |name|
  texts << "https://github.com/#{name}/r/pull/7"
  texts << "https://github.com/o/#{name}/pull/7"
end
%w[a _ é / ? # -].each { |tail| texts << "https://github.com/o/r/pull/12#{tail}" }
html = ["<p>see <code>https://github.com/o/r/pull/1</code> and https://github.com/o/r/pull/2</p><pre>https://github.com/o/r/pull/3</pre><p><a href='https://github.com/o/r/pull/4'>the PR</a></p>",
  "<div>https://github.com/o/r/pull/12<br>thanks</div><p>https://github.com/o/r/pull/13</p><p>for this</p>",
  "<code><a href='https://github.com/o/r/pull/1'>hidden</a></code><a href='https://github.com/o/r/pull/2?a&amp;b'>visible</a>"]
Github::PullRequestUrl::BLOCK_TAGS.each { |tag| html << "<#{tag}>https://github.com/o/r/pull/7</#{tag}>thanks" }
vectors={texts:texts.map { |text| {text:, references:Github::PullRequestUrl.extract(text).map { |r| [r.owner,r.repo,r.number.to_s] }, matches:Github::PullRequestUrl.pull_request_url?(text)} },
  html:html.map { |value| {html:value,text:Github::PullRequestUrl.non_code_text(value)} }}
File.write("/work/vectors/github_references.json",JSON.pretty_generate(vectors)+"\n")
puts "GitHub URL Rails oracle: #{vectors[:texts].size} extraction cases, #{vectors[:html].size} HTML cases; reference #{ENV.fetch('PARITY_REFERENCE_SHA')}"
