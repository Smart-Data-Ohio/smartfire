"""Own one reference network for one paired system replay, including teardown."""
import json
import subprocess


class ReferenceNetwork:
    def __init__(self, namespace, invocation, owner):
        self.name = f'{namespace}-{invocation}-parity-internal'
        self.owner = owner

    def close(self):
        result = subprocess.run(['docker', 'network', 'inspect', self.name],
                                capture_output=True, text=True)
        if result.returncode:
            return  # Startup may have failed before creating our network.
        network = json.loads(result.stdout)[0]
        if (network.get('Labels') or {}).get('parity.owner') != self.owner:
            raise RuntimeError(f'refusing to remove foreign reference network {self.name}')
        # Docker itself refuses to remove a network with any attached container.
        subprocess.run(['docker', 'network', 'rm', self.name], check=True,
                       capture_output=True, text=True)
