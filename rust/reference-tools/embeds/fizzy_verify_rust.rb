# The linked account's actual encrypted attribute type reads the Rust-written value.
data=JSON.parse(File.read(ARGV.fetch(0)))
raise 'Rust token did not round-trip through Rails' unless FizzyConnectedAccount.type_for_attribute('access_token').deserialize(data.fetch('ciphertext')) == data.fetch('token')
puts 'WS15e Fizzy token round-trip: Rails reads Rust ciphertext; Rust reads pinned Rails ciphertext'
