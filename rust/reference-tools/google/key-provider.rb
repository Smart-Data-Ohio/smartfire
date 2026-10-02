path=File.join(Gem::Specification.find_by_name('activerecord').full_gem_path,'lib/active_record/encryption/derived_secret_key_provider.rb')
puts File.read(path)
puts File.read(File.join(File.dirname(path),'key_provider.rb'))
