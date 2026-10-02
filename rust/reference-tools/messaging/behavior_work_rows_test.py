"""Discrimination of persisted permission/no-event and assignment assertions."""
import sqlite3
import unittest
from behavior_work_rows import assert_work_rows

class WorkRowsTest(unittest.TestCase):
    def database(self):
        c=sqlite3.connect(':memory:')
        c.executescript('''CREATE TABLE channel_threads(id,work_status,work_owner_id);
        CREATE TABLE work_thread_events(id,channel_thread_id,actor_id,event_type,from_status,to_status,from_owner_id,to_owner_id);
        CREATE TABLE agent_events(agent_id,event_type);
        CREATE TABLE messages(id,thread_id,markdown_source);
        INSERT INTO channel_threads VALUES(1,'planned',NULL);
        INSERT INTO messages VALUES(2,1,'Keep this history');''')
        self.addCleanup(c.close)
        return c
    def test_denied_assignment_discriminates_a_committed_owner_change(self):
        seed,candidate=self.database(),self.database()
        fixture={'thread_id':1,'history_message_id':2,'eligible_id':9,'eligible_agent_id':8}
        case='a member who cannot manage the thread cannot assign an agent'
        assert_work_rows(candidate,seed,case,fixture)
        candidate.execute('UPDATE channel_threads SET work_owner_id=9')
        with self.assertRaises(AssertionError):assert_work_rows(candidate,seed,case,fixture)
    def test_denied_assignment_discriminates_a_hidden_agent_event(self):
        seed,candidate=self.database(),self.database()
        fixture={'thread_id':1,'history_message_id':2,'eligible_id':9,'eligible_agent_id':8}
        case='a member who cannot manage the thread cannot assign an agent'
        candidate.execute("INSERT INTO agent_events VALUES(8,'work_assigned')")
        with self.assertRaises(AssertionError):assert_work_rows(candidate,seed,case,fixture)

if __name__=='__main__':unittest.main()
