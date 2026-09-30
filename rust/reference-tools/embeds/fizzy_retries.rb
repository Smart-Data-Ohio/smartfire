# Our real client against an isolated local TCP server: no fake Net::HTTP or external hosts.
require 'socket'
require 'net/http'
results=%w[GET DELETE POST].map do |method|
  server=TCPServer.new('127.0.0.1',51555)
  received=[]
  thread=Thread.new do
    loop do
      socket=server.accept
      line=socket.gets
      received << line.split.first
      headers={}
      while (line=socket.gets) && line!="\r\n"
        key,value=line.split(':',2); headers[key.downcase]=value.strip
      end
      socket.read(headers.fetch('content-length','0').to_i)
      if received.size>1
        socket.write("HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
      end
      socket.close
    end
  rescue IOError, Errno::EBADF
  end
  client=Fizzy::Client.new(token:'fixture-fizzy-retry',base_url:'http://127.0.0.1:51555')
  begin
    value=case method
    when 'GET' then client.identity
    when 'DELETE' then client.reopen_card('acc',12)
    when 'POST' then client.create_comment('acc',12,body:'Comment')
    end
    {method:method,requests:received.size,value:value}
  rescue Fizzy::Client::Error => error
    {method:method,requests:received.size,error:error.message}
  ensure
    server.close; thread.kill; thread.join
  end
end
File.write(ARGV.fetch(0),JSON.pretty_generate({reference:'d7c7de92',max_retries:Net::HTTP.new('app.fizzy.do').max_retries,cases:results})+"\n")
puts "WS15e Fizzy real Rails transport: GET/DELETE retry once after EOF; POST makes one attempt"
