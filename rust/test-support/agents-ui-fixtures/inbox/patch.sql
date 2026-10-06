DELETE FROM activity_items WHERE id=2;
DELETE FROM activity_items WHERE id=3;
DELETE FROM activity_items WHERE id=5;
DELETE FROM activity_items WHERE id=6;
DELETE FROM activity_items WHERE id=8;
DELETE FROM activity_items WHERE id=11;
DELETE FROM activity_items WHERE id=14;
INSERT INTO activity_items(id,created_at,event_type,handled_at,read_at,source_id,source_type,updated_at,user_id) VALUES(16,'2026-10-06 03:23:10.058247','mention',NULL,NULL,309456473,'Message','2026-10-06 03:23:10.058247',127326141);
UPDATE sqlite_sequence SET seq=16 WHERE rowid=3;
