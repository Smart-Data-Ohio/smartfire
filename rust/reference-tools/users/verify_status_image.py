#!/usr/bin/env python3
"""Check the plain reference image against the fresh pin archive, without overlays."""
from pathlib import Path
import os
import subprocess
root=Path(__file__).resolve().parents[2]
pin=Path(os.environ.get("CAMPFIRE_REFERENCE",root / "parity/.ci/reference"))
assert pin.is_dir()
image=os.environ.get("PARITY_IMAGE", "campfire-reference")
reference=(root / "parity/reference.sha").read_text().strip()
environment=subprocess.check_output(["docker", "image", "inspect", "--format",
                                     "{{range .Config.Env}}{{println .}}{{end}}", image], text=True)
assert [line.removeprefix("GIT_REVISION=") for line in environment.splitlines()
        if line.startswith("GIT_REVISION=")] == [reference], "status oracle requires the pinned reference image"
script=r'''
require 'digest'
harness={
  'config/resque-pool.yml'=>'resque-pool.yml',
  'config/initializers/parity_action_cable.rb'=>'parity_action_cable.rb',
  'config/initializers/parity_migration_timestamps.rb'=>'parity_migration_timestamps.rb',
  'Procfile'=>'Procfile'
}
paths=Dir.chdir('/pin') {Dir.glob('{app,config,db,lib,public,vendor,bin}/**/*',File::FNM_DOTMATCH).select{|path|File.file?(path)}}
paths+=Dir.chdir('/pin') {Dir.glob('test/fixtures/**/*',File::FNM_DOTMATCH).select{|path|File.file?(path)}}
paths+=%w[Gemfile Gemfile.lock .ruby-version]
paths-= ['bin/release']
paths.reject! {|path|path.match?(%r{\A(?:config/(?:credentials\.yml\.enc|master\.key|deploy.*\.yml)|bin/deploy(?:-.*)?)\z})}
paths=(paths+harness.keys).uniq
paths.each do |path|
  expected=harness.key?(path) ? File.join('/harness',harness[path]) : File.join('/pin',path)
  actual=File.join('/rails',path)
  raise "missing image source: #{path}" unless File.file?(actual)
  raise "image source drift: #{path}" unless Digest::SHA256.file(actual).hexdigest==Digest::SHA256.file(expected).hexdigest
end
puts "WS8br2 status image source verification: #{paths.length} Rails source/fixture/gem files checked; plain pinned reference #{ENV.fetch('PARITY_REFERENCE_SHA')}; 4 documented parity harness files checked; Dockerignore non-runtime paths omitted"
'''
subprocess.run(["docker","run","--rm","--network","none","-i","--label","parity.owner=ws8br2","--entrypoint","ruby",
                "-v",f"{pin}:/pin:ro","-v",f"{root}/parity/docker:/harness:ro",
                "-e",f"PARITY_REFERENCE_SHA={reference}", image,"-"],input=script,text=True,check=True)
