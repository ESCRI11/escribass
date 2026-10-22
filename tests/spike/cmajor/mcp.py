"""m5.1-spike: drive one escribass-mcp process a call at a time, so a later call can use an id an
earlier one returned. Nothing here is a second client of anything: it speaks the server's stdio."""
import json, os, subprocess, sys

REPO = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
MCP = f"{REPO}/target/release/escribass-mcp"
ENGINE = os.environ.get("SPIKE_ENGINE", f"{REPO}/engine/build/escribass_engine_artefacts/Release/escribass_engine")


class Session:
    def __init__(self, project, manifest, create=True):
        args = [MCP, "--manifest", manifest, "--engine", ENGINE, "--author", "human"]
        args += ["--create", project] if create else [project]
        self.p = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        self.n = 0
        self._send({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "m5.1-spike", "version": "1"}}})
        self._read(0)
        self._send({"jsonrpc": "2.0", "method": "notifications/initialized"})

    def _send(self, frame):
        self.p.stdin.write(json.dumps(frame) + "\n")
        self.p.stdin.flush()

    def _read(self, id_):
        while True:
            line = self.p.stdout.readline()
            if not line:
                raise RuntimeError(f"server closed before answering {id_}")
            frame = json.loads(line)
            if frame.get("id") == id_:
                return frame

    def call(self, tool, args, ok=True):
        self.n += 1
        self._send({"jsonrpc": "2.0", "id": self.n, "method": "tools/call",
                    "params": {"name": tool, "arguments": args}})
        frame = self._read(self.n)
        if "error" in frame:
            raise RuntimeError(f"{tool}: operator error: {frame['error']}")
        content = frame["result"].get("structuredContent", frame["result"])
        if ok and content.get("valid") is False:
            raise RuntimeError(f"{tool} refused: {json.dumps(content)[:2000]}")
        return content

    def close(self):
        self.p.stdin.close()
        return self.p.wait(timeout=60)
