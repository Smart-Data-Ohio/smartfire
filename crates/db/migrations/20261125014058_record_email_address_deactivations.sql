CREATE TABLE email_address_deactivations (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  user_id INTEGER NOT NULL,
  released_addresses TEXT NOT NULL
);
