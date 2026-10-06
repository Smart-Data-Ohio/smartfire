INSERT INTO action_text_rich_texts(id,body,created_at,name,record_id,record_type,updated_at) VALUES(935962059,'<p>Scheduled an event: Filtered event'||X'0a'
||'/rooms/654632876/events/439674038</p>','2026-10-06 03:23:22.145542','body',935962059,'Message','2026-10-06 03:23:22.145542');
DELETE FROM activity_items WHERE id=2;
DELETE FROM activity_items WHERE id=3;
DELETE FROM activity_items WHERE id=5;
DELETE FROM activity_items WHERE id=6;
DELETE FROM activity_items WHERE id=8;
DELETE FROM activity_items WHERE id=11;
DELETE FROM activity_items WHERE id=14;
INSERT INTO activity_items(id,created_at,event_type,handled_at,read_at,source_id,source_type,updated_at,user_id) VALUES(16,'2026-10-06 03:23:21.994138','mention',NULL,NULL,309456473,'Message','2026-10-06 03:23:21.994138',127326141);
INSERT INTO activity_items(id,created_at,event_type,handled_at,read_at,source_id,source_type,updated_at,user_id) VALUES(17,'2026-10-06 03:23:22.081197','event_invitation',NULL,NULL,439674038,'Event','2026-10-06 03:23:22.081197',127326141);
INSERT INTO activity_items(id,created_at,event_type,handled_at,read_at,source_id,source_type,updated_at,user_id) VALUES(18,'2026-10-06 03:23:22.083146','event_invitation',NULL,NULL,439674038,'Event','2026-10-06 03:23:22.083146',712064548);
INSERT INTO activity_items(id,created_at,event_type,handled_at,read_at,source_id,source_type,updated_at,user_id) VALUES(19,'2026-10-06 03:23:22.084316','event_invitation',NULL,NULL,439674038,'Event','2026-10-06 03:23:22.084316',773523953);
INSERT INTO event_attendances(id,created_at,event_id,response,updated_at,user_id) VALUES(937100614,'2026-10-06 03:23:22.057539',439674038,'going','2026-10-06 03:23:22.057539',149087659);
INSERT INTO event_references(id,created_at,event_id,message_id,updated_at) VALUES(1,'2026-10-06 03:23:22.240778',439674038,935962059,'2026-10-06 03:23:22.240778');
INSERT INTO events(id,cancelled_at,created_at,description,ends_at,meet_link,meet_link_requested,organizer_id,recurrence_rule,recurrence_until,reminded_at,room_id,series_id,starts_at,time_zone,title,updated_at,venue_room_id) VALUES(439674038,NULL,'2026-10-06 03:23:22.040068',NULL,NULL,NULL,0,149087659,NULL,NULL,NULL,654632876,NULL,'2026-10-08 03:23:22.019305','UTC','Filtered event','2026-10-06 03:23:22.040068',NULL);
UPDATE memberships SET unread_at='2026-10-06 03:23:22.140408', updated_at='2026-10-06 03:23:22.193124' WHERE id=146174848;
UPDATE memberships SET unread_at='2026-10-06 03:23:22.140408', updated_at='2026-10-06 03:23:22.193124' WHERE id=658335620;
UPDATE memberships SET unread_at='2026-10-06 03:23:22.140408', updated_at='2026-10-06 03:23:22.193124' WHERE id=741303990;
UPDATE memberships SET last_read_message_id=935962059, updated_at='2026-10-06 03:23:22.199959' WHERE id=897066373;
UPDATE memberships SET unread_at='2026-10-06 03:23:22.140408', updated_at='2026-10-06 03:23:22.193124' WHERE id=1021579687;
UPDATE memberships SET unread_at='2026-10-06 03:23:22.140408', updated_at='2026-10-06 03:23:22.193124' WHERE id=1021579688;
INSERT INTO message_search_index_content(id,c0) VALUES(935962059,'Scheduled an event: Filtered event'||X'0a'
||'/rooms/654632876/events/439674038');
UPDATE message_search_index_data SET block=x'813b8b10' WHERE id=1;
UPDATE message_search_index_data SET block=x'000000000210814d000e0501010101010201010301010401010601010701010801010901010a01010b01010c01010d01010e01010002100102110102' WHERE id=10;
INSERT INTO message_search_index_data(id,block) VALUES(1924145348609,x'0000006f0a3034333936373430333883bea6cb4b020a010936353436333238373683bea6cb4b02080102616e83bea6cb4b020301056576656e7483bea6cb4b06040405010666696c74657283bea6cb4b02050104726f6f6d83bea6cb4b020701077363686564756c83bea6cb4b02020412120b100f0d');
INSERT INTO message_search_index_docsize(id,sz) VALUES(935962059,x'09');
INSERT INTO message_search_index_idx(segid,term,pgno) VALUES(14,X'',2);
INSERT INTO messages(id,"action",board_post_opener,client_message_id,created_at,creator_id,edited_at,embeds_suppressed,forward_note,forwarded_at,forwarded_from_message_id,forwarded_markdown,markdown_source,reply_notify_author,reply_target_deleted_at,reply_to_message_id,room_id,stream_broadcast_at,streaming,streaming_updated_at,system_note,thread_id,updated_at) VALUES(935962059,0,0,'475765a6-c8e4-4acf-a063-5d7ed78fd4c8','2026-10-06 03:23:22.140408',149087659,NULL,0,NULL,NULL,NULL,0,'Scheduled an event: Filtered event'||X'0a'
||'/rooms/654632876/events/439674038',1,NULL,NULL,654632876,NULL,0,NULL,0,NULL,'2026-10-06 03:23:22.148265');
UPDATE rooms SET updated_at='2026-10-06 03:23:22.172178' WHERE id=654632876;
UPDATE sqlite_sequence SET seq=19 WHERE rowid=3;
UPDATE sqlite_sequence SET seq=1 WHERE rowid=17;
UPDATE sqlite_sequence SET seq=935962059 WHERE rowid=32;
UPDATE sqlite_sequence SET seq=935962059 WHERE rowid=61;
UPDATE sqlite_sequence SET seq=937100614 WHERE rowid=64;
UPDATE sqlite_sequence SET seq=439674038 WHERE rowid=65;
