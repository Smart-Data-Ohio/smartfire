INSERT INTO action_text_rich_texts(id,body,created_at,name,record_id,record_type,updated_at) VALUES(935962058,'<table>'||X'0a'
||'<thead>'||X'0a'
||'<tr>'||X'0a'
||'<th>Keep</th>'||X'0a'
||'</tr>'||X'0a'
||'</thead>'||X'0a'
||'<tbody>'||X'0a'
||'<tr>'||X'0a'
||'<td>row</td>'||X'0a'
||'</tr>'||X'0a'
||'</tbody>'||X'0a'
||'</table>'||X'0a'
||'<pre><code class="language-ruby">puts :forwarded'||X'0a'
||'</code></pre>','2026-03-02 16:00:00','body',935962058,'Message','2026-03-02 16:00:00');
UPDATE memberships SET unread_at='2026-03-02 16:00:00' WHERE id=146174848;
UPDATE memberships SET unread_at='2026-03-02 16:00:00' WHERE id=658335620;
UPDATE memberships SET last_read_message_id=935962058 WHERE id=741303990;
UPDATE memberships SET unread_at='2026-03-02 16:00:00' WHERE id=897066373;
UPDATE memberships SET unread_at='2026-03-02 16:00:00' WHERE id=1021579687;
UPDATE memberships SET unread_at='2026-03-02 16:00:00' WHERE id=1021579688;
INSERT INTO message_search_index_content(id,c0) VALUES(935962058,'Keep'||X'0a0a'
||'row'||X'0a0a'
||'puts :forwarded');
UPDATE message_search_index_data SET block=x'813a8b07' WHERE id=1;
UPDATE message_search_index_data SET block=x'00000000020f814c000d0501010101010201010301010401010601010701010801010901010a01010b01010c01010d01010002100102110102' WHERE id=10;
INSERT INTO message_search_index_data(id,block) VALUES(1786706395137,x'000000390830666f727761726483bea6cb4a020501046b65657083bea6cb4a0202010370757483bea6cb4a02040103726f7783bea6cb4a020304100d0c');
INSERT INTO message_search_index_docsize(id,sz) VALUES(935962058,x'04');
INSERT INTO message_search_index_idx(segid,term,pgno) VALUES(13,X'',2);
INSERT INTO messages(id,"action",board_post_opener,client_message_id,created_at,creator_id,edited_at,embeds_suppressed,forward_note,forwarded_at,forwarded_from_message_id,forwarded_markdown,markdown_source,reply_notify_author,reply_target_deleted_at,reply_to_message_id,room_id,stream_broadcast_at,streaming,streaming_updated_at,system_note,thread_id,updated_at) VALUES(935962058,0,0,'system-forward-source','2026-03-02 16:00:00',773523953,NULL,0,NULL,NULL,NULL,0,'| Keep |'||X'0a'
||'| --- |'||X'0a'
||'| row |'||X'0a0a'
||'```ruby'||X'0a'
||'puts :forwarded'||X'0a'
||'```',1,NULL,NULL,654632876,NULL,0,NULL,0,NULL,'2026-03-02 16:00:00');
UPDATE sqlite_sequence SET seq=935962058 WHERE rowid=32;
UPDATE sqlite_sequence SET seq=935962058 WHERE rowid=61;
