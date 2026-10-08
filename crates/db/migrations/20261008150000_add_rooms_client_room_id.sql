ALTER TABLE "rooms" ADD COLUMN "client_room_id" varchar;
CREATE UNIQUE INDEX "index_rooms_on_creator_id_and_client_room_id" ON "rooms" ("creator_id", "client_room_id") WHERE client_room_id IS NOT NULL;
