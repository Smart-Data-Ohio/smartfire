require "restricted_http/private_network_guard"
inputs=JSON.parse(File.read(File.join(ENV.fetch("PARITY_WORK"),"reference-tools/agents/private_guard_case_inputs.json")))
original=Resolv.method(:getaddresses)
cases=inputs.map do |row|
  results=if row["kind"]=="private_ip"
    row.fetch("addresses").map{|address|{address:address,private:RestrictedHTTP::PrivateNetworkGuard.private_ip?(address)}}
  else
    row.fetch("hosts").each_with_index.map do |host,index|
      answers=row.fetch("answer_sets",nil)&.fetch(index)||row.fetch("answers",[])
      Resolv.define_singleton_method(:getaddresses){|_host|answers}
      begin;{host:host,answers:answers,address:RestrictedHTTP::PrivateNetworkGuard.resolve(host)}
      rescue=>error;{host:host,answers:answers,error:error.class.name};end
    end
  end
  {name:row.fetch("name"),kind:row.fetch("kind"),results:results}
end
Resolv.define_singleton_method(:getaddresses,original)
puts JSON.pretty_generate(reference_pin:ENV.fetch("PARITY_REFERENCE_SHA")[0, 8],cases:cases)
