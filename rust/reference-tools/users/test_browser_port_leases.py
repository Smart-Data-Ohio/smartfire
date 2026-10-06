"""Real kernel leases: separate owners, overlap, lifetime and ephemeral safety."""
import multiprocessing
import socket
import unittest
from pathlib import Path
from browser_port_leases import reserve, reserve_system_ports


def child_ports(pipe, base):
    with reserve(base) as lease:
        pipe.send(lease.ports)
        pipe.recv()


class BrowserPortLeases(unittest.TestCase):
    def test_paired_system_hosts_use_disjoint_non_ephemeral_leases(self):
        low, high = map(int, Path('/proc/sys/net/ipv4/ip_local_port_range').read_text().split())
        with reserve_system_ports('work') as first, reserve_system_ports('work') as second, reserve() as originals:
            self.assertTrue(all(not low <= port <= high for port in first.ports))
            self.assertTrue(set(first.ports).isdisjoint(second.ports))
            self.assertTrue(set(first.ports).isdisjoint(originals.ports))
            for port in first.ports + second.ports:
                with socket.socket() as server:
                    server.bind(('127.0.0.1', port))

    def test_outbound_connections_cannot_acquire_a_reserved_port(self):
        low, high = map(int, Path('/proc/sys/net/ipv4/ip_local_port_range').read_text().split())
        with reserve() as lease, socket.socket() as listener:
            self.assertTrue(all(not low <= port <= high for port in lease.ports))
            listener.bind(('127.0.0.1', 0))
            listener.listen()
            with socket.create_connection(listener.getsockname()) as outbound:
                self.assertTrue(low <= outbound.getsockname()[1] <= high)
                self.assertNotIn(outbound.getsockname()[1], lease.ports)
                # Binding the real servers remains possible throughout the lease.
                for port in lease.ports:
                    with socket.socket() as server:
                        server.bind(('127.0.0.1', port))

    def test_processes_and_overlapping_ranges_cannot_share_a_lease(self):
        with reserve(30000) as first:
            context = multiprocessing.get_context('fork')
            parent, child = context.Pipe()
            process = context.Process(target=child_ports, args=(child, first.ports[0] + 1))
            process.start()
            try:
                self.assertTrue(parent.poll(5), 'child must allocate without a startup wait')
                other = parent.recv()
                self.assertTrue(set(first.ports).isdisjoint(other))
                parent.send('release')
                process.join(5)
                self.assertEqual(process.exitcode, 0)
            finally:
                if process.is_alive():
                    process.terminate()
                    process.join()

    def test_released_lease_is_available_without_a_stale_lock(self):
        with reserve(30000) as first:
            ports = first.ports
            with reserve(ports[0]) as second:
                self.assertTrue(set(ports).isdisjoint(second.ports))
        with reserve(ports[0]) as reopened:
            self.assertEqual(reopened.ports, ports)

    def test_unsafe_override_is_rejected_before_startup(self):
        with self.assertRaisesRegex(ValueError, 'overlaps ephemeral'):
            reserve(52710)


if __name__ == '__main__':
    unittest.main()
