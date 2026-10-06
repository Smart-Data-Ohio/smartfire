"""Restore the isolated original fixture rows between system declarations."""

def quoted(name):
    return '"' + name.replace('"', '""') + '"'

def snapshot(db):
    state = {}
    for _, name, kind, *_ in db.execute('PRAGMA table_list').fetchall():
        if kind not in ('table', 'virtual') or name.startswith('sqlite_'):
            continue
        query = 'SELECT ' + ('rowid AS ledger_rowid, ' if kind == 'virtual' else '') + '* FROM ' + quoted(name)
        cursor = db.execute(query)
        state[name] = ([column[0] for column in cursor.description], cursor.fetchall(), kind)
    state['sqlite_sequence'] = (['name', 'seq'], db.execute('SELECT name,seq FROM sqlite_sequence').fetchall(), 'table')
    return state

def restore(db, state):
    # The fixture connection owns a transaction; never replace the live WAL DB.
    # Virtual roots clear their own FTS shadow tables and keep explicit rowids.
    for name in state:
        db.execute('DELETE FROM ' + quoted(name))
    for name, (columns, rows, kind) in state.items():
        names = ['rowid' if kind == 'virtual' and column == 'ledger_rowid' else column for column in columns]
        sql = 'INSERT INTO ' + quoted(name) + '(' + ','.join(map(quoted, names)) + ') VALUES (' + ','.join('?' for _ in names) + ')'
        db.executemany(sql, rows)
    # Explicit fixture inserts can grow sequences after their snapshot was
    # inserted; restore the original sequence last, as a fresh fixture load.
    db.execute('DELETE FROM sqlite_sequence')
    db.executemany('INSERT INTO sqlite_sequence(name,seq) VALUES (?,?)', state['sqlite_sequence'][1])
