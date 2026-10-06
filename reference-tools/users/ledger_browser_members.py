"""Isolated fixtures and persisted observations for the pinned member originals."""
import hashlib

CASES = [
    'member-inline', 'member-plain', 'member-modifiers', 'member-range',
    'member-range-box', 'member-departed-profile', 'member-exits',
    'member-exit-focus', 'member-fallback-focus', 'member-uncheck-focus',
    'member-mobile-departed', 'member-desktop-departed', 'member-menu-escape',
    'member-status', 'member-space', 'member-rerender', 'member-star-menu',
    'member-long-press', 'channel-online', 'channel-agent', 'channel-tabs',
    'channel-phone', 'motion-default', 'motion-drawer', 'motion-members',
    'motion-directory', 'motion-sticky', 'motion-menu', 'motion-scroll',
    'motion-first-reveal', 'motion-reopen-current',
]

MUTATIONS = {
    'member-count': 'member-plain',
    'member-bar-scope': 'member-plain',
    'member-hidden-checkbox': 'member-modifiers',
    'member-inline-zero': 'member-inline',
    'member-inline-hidden': 'member-inline',
    'member-inline-out-of-frame': 'member-inline',
    'member-focus': 'member-exit-focus',
    'member-rerender': 'member-rerender',
    'channel-presence': 'channel-online',
    'channel-composer': 'channel-phone',
    'motion-member-shift': 'motion-members',
    'motion-sticky-out-of-frame': 'motion-sticky',
    'motion-menu-clamp': 'motion-menu',
    'motion-scroll-retention': 'motion-scroll',
    'motion-reopen-focus': 'motion-scroll',
}

CONTROL_CASES = list(dict.fromkeys(MUTATIONS.values()))


def environment(labels, root):
    # MotionTest pins the server's test-environment marker, rather than a DOM edit.
    return {'RAILS_ENV': 'test'}


def prepare(db, labels, root):
    # The fixture mutations below occur only in the copied, disposable seed.
    labels['users.panel_bot'] = 9100010001
    labels['rooms.planning'] = 9100010002
    labels['rooms.far'] = 9100010003
    labels['ledger.memberships'] = {
        user: dict(zip([c[0] for c in db.execute('SELECT * FROM memberships LIMIT 0').description],
                       db.execute('SELECT * FROM memberships WHERE room_id=? AND user_id=?',
                                  [labels['rooms.designers'], labels['users.' + user]]).fetchone()))
        for user in ['david', 'jason', 'jz', 'kevin']
    }


def fixture(db, case, labels, root):
    # The shared runner restores the original foundation before class setup.
    # All declaration-specific writes happen live at their original source point.
    if case.startswith(('member-', 'channel-')):
        db.execute('DELETE FROM workspace_presence_leases')


def check_state(db, requests, labels, root):
    for request in requests:
        if request['kind'] == 'direct':
            expected = sorted(labels['users.' + user] for user in request['users'])
            actual = sorted(row[0] for row in db.execute('SELECT user_id FROM memberships WHERE room_id=?', [request['room']]))
            assert actual == expected, ('exact original recipients', actual, expected)
            key = 'dm:' + hashlib.sha256(','.join(map(str, expected)).encode()).hexdigest()
            assert db.execute('SELECT id FROM rooms WHERE direct_member_key=? AND deleted_at IS NULL', [key]).fetchone() == (request['room'],)
        elif request['kind'] == 'unread':
            assert db.execute('SELECT unread_at FROM memberships WHERE user_id=? AND room_id=?',
                              [labels['users.kevin'], labels['rooms.designers']]).fetchone()[0] is not None
        elif request['kind'] == 'session':
            assert db.execute('SELECT count(*) FROM sessions WHERE user_id=?', [labels['users.kevin']]).fetchone()[0] > 0
        else:
            raise AssertionError(request)

# The tools-only host also clears the real renderer cache between originals.
from ledger_browser_lifecycle import host_command
