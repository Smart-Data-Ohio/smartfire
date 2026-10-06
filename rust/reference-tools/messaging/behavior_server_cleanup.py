"""Always stop the app under test, killing it if it doesn't exit."""
import subprocess


def stop_behavior_server(process):
    if process is None:
        return
    try:
        process.terminate()
        process.wait(timeout=15)
    except (OSError, subprocess.TimeoutExpired):
        process.kill()
        process.wait()
