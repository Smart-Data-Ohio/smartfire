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
        CREATE TABLE messages(id,thread_id,markdown_source,client_message_id);
        INSERT INTO channel_threads VALUES(1,'planned',NULL);
        INSERT INTO messages VALUES(2,1,'Keep this history','work-history');''')
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

    def test_refusal_counts_events_in_other_threads(self):
        seed,candidate=self.database(),self.database()
        fixture={'thread_id':1,'history_message_id':2,'eligible_id':9,'eligible_agent_id':8}
        case='a member who cannot manage the thread cannot assign an agent'
        candidate.execute("INSERT INTO work_thread_events VALUES(3,99,7,'work_update','planned','in_progress',NULL,NULL)")
        with self.assertRaisesRegex(AssertionError,'work-event-count:'):
            assert_work_rows(candidate,seed,case,fixture)

    def conversion(self):
        seed,candidate=self.database(),self.database()
        seed.execute('UPDATE channel_threads SET work_status=NULL')
        candidate.execute('UPDATE channel_threads SET work_owner_id=712064548')
        for db in [seed,candidate]:
            db.execute("INSERT INTO work_thread_events VALUES(1,99,7,'work_update','planned','in_progress',NULL,NULL)")
        candidate.executescript("""INSERT INTO work_thread_events VALUES(2,1,773523953,'work_update',NULL,'planned',NULL,NULL);
            INSERT INTO work_thread_events VALUES(3,1,773523953,'work_assignment','planned','planned',NULL,712064548);""")
        return seed,candidate,{'thread_id':1,'history_message_id':2,'eligible_id':9,'eligible_agent_id':8}

    def test_conversion_uses_global_delta_with_existing_foreign_events(self):
        seed,candidate,fixture=self.conversion()
        assert_work_rows(candidate,seed,'converts a thread to work, assigns an eligible owner, and keeps an audit trail',fixture)

    def test_conversion_rejects_extra_foreign_event(self):
        seed,candidate,fixture=self.conversion()
        candidate.execute("INSERT INTO work_thread_events VALUES(4,99,7,'work_update','planned','in_progress',NULL,NULL)")
        with self.assertRaisesRegex(AssertionError,'work-event-count:'):
            assert_work_rows(candidate,seed,'converts a thread to work, assigns an eligible owner, and keeps an audit trail',fixture)

    def test_history_lookup_rejects_rewritten_client_id(self):
        seed,candidate=self.database(),self.database()
        fixture={'thread_id':1,'history_message_id':2,'eligible_id':9,'eligible_agent_id':8}
        candidate.execute("UPDATE messages SET client_message_id='corrupted-work-history'")
        # The former by-ID source/thread assertion still passes.
        self.assertEqual(candidate.execute('SELECT thread_id,markdown_source FROM messages WHERE id=2').fetchone(),(1,'Keep this history'))
        with self.assertRaisesRegex(AssertionError,'work-history identity:'):
            assert_work_rows(candidate,seed,'ordinary thread fields remain separate from work tracking',fixture)

    def test_history_lookup_requires_original_identity(self):
        seed,candidate=self.database(),self.database()
        fixture={'thread_id':1,'history_message_id':2,'eligible_id':9,'eligible_agent_id':8}
        candidate.execute("UPDATE messages SET client_message_id='moved'")
        candidate.execute("INSERT INTO messages VALUES(3,1,'Keep this history','work-history')")
        with self.assertRaisesRegex(AssertionError,'work-history identity:'):
            assert_work_rows(candidate,seed,'ordinary thread fields remain separate from work tracking',fixture)

if __name__=='__main__':unittest.main()
