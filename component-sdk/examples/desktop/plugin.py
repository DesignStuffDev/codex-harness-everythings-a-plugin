"""Optional presentation component; Codex owns execution and saved sessions."""

from codex_component_sdk import Plugin, PluginError
from desktop_ui.gateway import Gateway
import signal
import sys

app = Plugin()
gateway = None


@app.on_start
def start(initialization):
    def terminate(signum, frame):
        raise SystemExit(0)

    signal.signal(signal.SIGTERM, terminate)
    signal.signal(signal.SIGINT, terminate)


@app.handle("presentation", "desktop", "launch")
def launch(params, context):
    global gateway
    try:
        gateway = Gateway(params)
    except (ValueError, OSError, RuntimeError):
        raise PluginError(
            "Desktop could not start; check the Codex executable and port"
        ) from None
    result = {
        "url": gateway.url,
        "app_server_pid": gateway.bridge.child.pid,
        "gateway_pid": __import__("os").getpid(),
    }
    shutdown_requested = context.watch_shutdown()
    context.emit({"method": "presentation/ready", "params": result})
    disconnected = False
    while not shutdown_requested.wait(0.1):
        if not gateway.bridge.reader.is_alive():
            disconnected = True
            break
    try:
        gateway.close()
    except (OSError, RuntimeError):
        raise PluginError("Codex cleanup failed; write outcome unknown") from None
    if disconnected:
        # Even a zero exit before a shutdown request is a lost session.
        raise PluginError("Codex disconnected unexpectedly; write outcome unknown")
    return result


@app.on_shutdown
def shutdown(initialization):
    if gateway:
        try:
            gateway.close()
        except (OSError, RuntimeError):
            # SDK cleanup exceptions are otherwise diagnostic-only. A failed
            # durability boundary must also produce a nonzero process exit.
            print("Codex cleanup failed; write outcome unknown", file=sys.stderr)
            raise SystemExit(1) from None
