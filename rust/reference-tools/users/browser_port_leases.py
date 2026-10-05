"""Host-wide leases for the original-browser servers (Linux abstract sockets).

TCP listener handover is not supported by both servers. Instead reserve ports
outside the kernel's outbound ephemeral range. Abstract socket names coordinate
all runner processes/worktrees, and stay bound until the owner exits, including
the entire Rails and Rust startup window. No filesystem lock or stale lock file
is involved. The finite allocation scan is not a server-startup retry.
"""
import errno
import os
import socket
from pathlib import Path

DEFAULT_BASE = 24000
BLOCKS = 64
WIDTH = 3


def reserve_system_ports(scenario):
    # Share host-wide coordination with the original-assertion wrappers. Keep
    # all three sockets available through Rails boot and candidate startup;
    # an outbound byte-forwarder connection cannot take a leased server port.
    modes = ['pages', 'budget', 'work', 'inbox', 'inbox-filter']
    return reserve(int(os.environ.get('WS11UI_SYSTEM_PORT_BASE', str(DEFAULT_BASE))),
                   preferred=32 + modes.index(scenario))


class PortLease:
    def __init__(self, ports, locks):
        self.ports = ports
        self.locks = locks

    def close(self):
        for lock in self.locks:
            lock.close()
        self.locks.clear()

    def __enter__(self):
        return self

    def __exit__(self, *unused):
        self.close()


def reserve(base=DEFAULT_BASE, preferred=0):
    low, high = map(int, Path('/proc/sys/net/ipv4/ip_local_port_range').read_text().split())
    end = base + WIDTH * BLOCKS - 1
    if base < 1024 or end > 65535 or not (end < low or base > high):
        raise ValueError(f'browser lease range {base}-{end} overlaps ephemeral {low}-{high} or is invalid')
    for index in range(BLOCKS):
        start = base + WIDTH * ((preferred + index) % BLOCKS)
        ports = list(range(start, start + WIDTH))
        locks = []
        try:
            for port in ports:
                lock = socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM)
                locks.append(lock)
                # Individual names also reject partially overlapping ranges.
                lock.bind(f'\0smartfire-original-browser-port:{port}')
                with socket.socket() as check:
                    check.bind(('127.0.0.1', port))
            return PortLease(ports, locks)
        except OSError as error:
            for lock in locks:
                lock.close()
            if error.errno != errno.EADDRINUSE:  # occupied lease or existing listener
                raise
    raise RuntimeError(f'all {BLOCKS} browser port leases are occupied')
