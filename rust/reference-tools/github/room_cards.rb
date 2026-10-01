# Reuse the pinned card oracle's real rows and fragment source, then add the empty container.
load '/work/scripts/cards.rb'
cases = JSON.parse(File.read('/work/vectors/github_cards.json'))
names = %w[public loading private unknown error checks_ discuss_link]
selected = cases.select { |c| names.include?(c.fetch('name')) }
Github::PullRequestReference.delete_all
message = Message.find(818)
empty = ApplicationController.render(partial: 'github/pull_requests/cards', locals: { message: })
selected << { name: 'no_cards', cards: empty }
File.write('/work/vectors/github_room_cards.json', JSON.pretty_generate(selected)+"\n")
puts "GitHub room-card Rails oracle: #{selected.size} card-container cases through actual associations; reference d7c7de92"
