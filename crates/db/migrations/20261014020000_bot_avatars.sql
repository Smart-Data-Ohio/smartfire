-- The bot avatar a bot's creator picked from the bot-avatars library: its body, colour, face and
-- what it wears. The SPA draws it; the bot's attached avatar is its still render for everything
-- else. Bots without a row keep their icon or uploaded picture.
CREATE TABLE bot_avatars (
  user_id INTEGER PRIMARY KEY NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  shape TEXT NOT NULL,
  color TEXT NOT NULL,
  face TEXT NOT NULL,
  accessory TEXT,
  created_at datetime(6) NOT NULL,
  updated_at datetime(6) NOT NULL
);
