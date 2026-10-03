"""Always stop the Rails instance after attempting candidate shutdown."""
import subprocess


def stop_behavior_servers(process, reference, port, *, cwd, env, log):
    try:
        if process is not None:
            try:
                process.terminate()
                process.wait(timeout=15)
            except (OSError, subprocess.TimeoutExpired):
                process.kill()
                process.wait()
    finally:
        subprocess.run([reference, 'down', '--port', str(port)], cwd=cwd,
                       env=env, stdout=log, stderr=log, check=True)
