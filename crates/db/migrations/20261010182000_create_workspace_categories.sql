CREATE TABLE workspace_categories (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    name TEXT NOT NULL,
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
CREATE INDEX index_workspace_categories_on_position ON workspace_categories (position);
ALTER TABLE rooms ADD COLUMN workspace_category_id INTEGER REFERENCES workspace_categories(id) ON DELETE SET NULL;
ALTER TABLE rooms ADD COLUMN workspace_position INTEGER;
CREATE INDEX index_rooms_on_workspace_category_id_and_position ON rooms (workspace_category_id, workspace_position);
