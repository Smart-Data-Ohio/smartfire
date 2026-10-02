require 'json'
Current.user=User.find(127326141)
room=Room.find(699448326)
renderer=ApplicationController.renderer.new(http_host:'campfire.test',https:false)
messages=[]
cases=[]
create_message=->(name) { m=room.messages.create!(creator:Current.user,markdown_source:'provider reference',client_message_id:"provider-#{name}");messages<<m;m }
pr_cases=[
  {label:'loading',state:nil,title:nil},
  {label:'open',state:'open',title:'Title <&> "quoted"',author_login:'author<&>',author_avatar_url:'https://example.test/avatar?a=1&b=2',base_branch:'main',head_branch:'feature<&>',review_decision:'approved',check_status:'passing',github_updated_at:Time.current-1.hour,payload:{base:{repo:{full_name:'Example/Repo'}}}},
  {label:'merged',state:'merged',title:'Merged',review_decision:'changes_requested',check_status:'failing'},
  {label:'closed',state:'closed',title:'Closed',review_decision:'review_required',check_status:'pending'},
  {label:'draft',state:'draft',title:'Draft',html_url:'https://github.com/Cased/Repository/pull/2'},
  {label:'error',state:'open',title:'Hidden by error',fetch_error:'HTTP <&> 404'},
  {label:'thread',state:'open',title:'Discussion',mapped:true},
  {label:'reply',state:'open',title:'Reply card',thread:true},
  {label:'invalid_display',state:'open',title:'Fallback display',payload:{base:{repo:{full_name:'bad / name'}}}},
  {label:'private',state:'open',title:'PRIVATE TITLE',private:true},
  {label:'unknown',state:'open',title:'UNKNOWN TITLE',private:nil}
]
pr_cases.each_with_index do |attributes,i|
  label=attributes.delete(:label);mapped=attributes.delete(:mapped);threaded=attributes.delete(:thread)
  m=create_message.call(label)
  if mapped || threaded
    t=ChannelThread.create!(room:,creator:Current.user,name:'PR discussion')
    m.update_columns(thread_id:t.id) if threaded
  end
  attributes={private:false}.merge(attributes)
  pr=Github::PullRequest.create!({owner:'provider-owner',repo:"repo-#{i}",number:i+1,fetched_at:Time.current,fetch_requested_at:Time.current}.merge(attributes))
  Github::PullRequestReference.create!(message:m,pull_request:pr)
  Github::PullRequestThread.create!(room:,pull_request:pr,channel_thread:t) if mapped
  html=renderer.render(partial:'github/pull_requests/cards',locals:{message:Message.with_rendering_details.find(m.id)})
  cases<<{label:,message_id:m.id,kind:'github',html:}
end
[
  {label:'generic_title',url:'https://page.example.test/post#my-fragment',title:'Title <&>',description:'Description "quoted"',site_name:'Site <&>',image_url:'https://img.example.test/image?a=1&b=2'},
  {label:'generic_description',url:'https://page.example.test/description',description:'Description only'},
  {label:'generic_image_only',url:'https://page.example.test/image',image_url:'https://img.example.test/image'},
  {label:'linkedin_player',url:'https://www.linkedin.com/feed/update/urn:li:activity:1234567?my=query#my-fragment',title:'LinkedIn <&>',description:'Post excerpt',image_url:'https://img.example.test/post'},
  {label:'linkedin_posts',url:'https://linkedin.com/posts/author-post',title:'Post title'},
  {label:'linkedin_chip',url:'https://www.linkedin.com/feed/update/urn:li:share:1234567'},
  {label:'suppressed',url:'https://page.example.test/suppressed',title:'Suppressed',suppressed:true}
].each do |attributes|
  label=attributes.delete(:label);url=attributes.delete(:url);suppressed=attributes.delete(:suppressed)
  m=create_message.call(label)
  m.update_columns(embeds_suppressed:true) if suppressed
  embed=LinkEmbed.create!({normalized_url:LinkEmbed.normalize_url(url),expires_at:Time.current+1.day,fetched_at:Time.current}.merge(attributes))
  LinkEmbedReference.create!(message:m,link_embed:embed,url:,position:0)
  kind=embed.linkedin? ? 'linkedin' : 'embed'
  html=renderer.render(partial:kind=='linkedin' ? 'linkedin/posts/cards' : 'link_embeds/cards',locals:{message:Message.with_rendering_details.find(m.id)})
  cases<<{label:,message_id:m.id,kind:,html:}
end
ids=messages.map(&:id).join(',')
selects={
 'messages'=>"id IN (#{ids})",'action_text_rich_texts'=>"record_type='Message' AND record_id IN (#{ids})",
 'github_pull_requests'=>"id IN (SELECT github_pull_request_id FROM github_pull_request_references WHERE message_id IN (#{ids}))",
 'github_pull_request_references'=>"message_id IN (#{ids})",
 'channel_threads'=>"name='PR discussion'",'github_pull_request_threads'=>"github_pull_request_id IN (SELECT github_pull_request_id FROM github_pull_request_references WHERE message_id IN (#{ids}))",
 'link_embeds'=>"id IN (SELECT link_embed_id FROM link_embed_references WHERE message_id IN (#{ids}))",'link_embed_references'=>"message_id IN (#{ids})"
}
rows=selects.to_h{|table,where|[table,ActiveRecord::Base.connection.select_all("SELECT * FROM #{table} WHERE #{where} ORDER BY id").to_a]}
File.write(ARGV.fetch(0),JSON.pretty_generate(reference:'d7c7de92',rows:,cases:)+"\n")
puts "WS8bm2 provider Rails oracle: #{pr_cases.size} GitHub containers; 7 embed/LinkedIn containers; #{rows.size} fixture tables"
