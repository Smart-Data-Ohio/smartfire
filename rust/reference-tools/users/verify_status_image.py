#!/usr/bin/env python3
"""Check the owned image against the fresh pin archive and the ten approved source files."""
from pathlib import Path
import os
import subprocess
root=Path(__file__).resolve().parents[2]
pin=Path(os.environ.get("CAMPFIRE_REFERENCE",root / "parity/.ci/reference"))
assert pin.is_dir()
script=r'''
require 'digest'
require 'json'
base='/approved'
ledger=JSON.parse(File.read(File.join(base,'source-hashes.json')))
approved=ledger.to_h {|path,hash|[(path.start_with?('app/','config/') ? path : 'app/views/'+path),[File.join(base,path),hash]]}
paths=Dir.chdir('/pin') {Dir.glob('{app,config,db,lib,public,vendor,test,bin}/**/*',File::FNM_DOTMATCH).select{|path|File.file?(path)}}
paths+=%w[Gemfile Gemfile.lock .ruby-version]
paths-= ['bin/release']
paths=(paths+approved.keys).uniq
paths.each do |path|
  expected=approved.key?(path) ? approved[path][0] : File.join('/pin',path)
  actual=File.join('/rails',path)
  raise "missing image source: #{path}" unless File.file?(actual)
  raise "image source drift: #{path}" unless Digest::SHA256.file(actual).hexdigest==Digest::SHA256.file(expected).hexdigest
  raise "approved ledger drift: #{path}" if approved.key?(path) && Digest::SHA256.file(expected).hexdigest!=approved[path][1]
end
puts "WS8br2 status image source verification: #{paths.length} Rails source/fixture/gem files checked; exactly 10 approved 2e20b24c inputs, all others d7c7de92; non-runtime bin/release omitted"
'''
subprocess.run(["docker","run","--rm","--network","none","-i","--label","parity.owner=ws8br2","--entrypoint","ruby",
                "-v",f"{pin}:/pin:ro","-v",f"{root}/reference-tools/users/post-pin:/approved:ro",
                "ws8br2-reference:d7c7de92-status-2e20b24c","-"],input=script,text=True,check=True)
