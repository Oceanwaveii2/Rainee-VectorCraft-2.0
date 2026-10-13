"""Mocked stdio/RPC tests: never start Codex or consume model inference."""
import base64
import io
import json
import queue
import sys
import threading
import unittest
from unittest.mock import patch

from integrations import codex_bridge as bridge


class Input:
    def __init__(self, first=None):
        self.lines = queue.Queue()
        if first is not None:
            self.send(first)

    def send(self, value):
        self.lines.put(bridge.encode(value) + "\n")

    def readline(self, limit=-1):
        return self.lines.get(timeout=3)


class RecordingOutput(io.StringIO):
    def __init__(self, callback=None):
        super().__init__()
        self.callback = callback
        self.events = []

    def write(self, line):
        event = json.loads(line)
        self.events.append(event)
        result = super().write(line)
        if self.callback:
            self.callback(event)
        return result


class FakeClient(bridge.EditorRun):
    instances = []
    signed_in = True
    behavior = "success"

    def __init__(self, workspace, tools, handler):
        self.workspace, self.tools, self.handler = workspace, tools, handler
        self.turn_lock = threading.Lock()
        self.events = queue.Queue()
        self.active = None
        self.process = object()
        self.calls = []
        self.closed = False
        self.instances.append(self)

    def account(self):
        return {"signedIn": self.signed_in}

    def models(self):
        return [{"model": "mock-model", "isDefault": True}]

    def notification(self, method, **params):
        self.events.put({"method": method, "params": {"threadId": "thread", **params}})

    def rpc(self, method, params, timeout=30):
        self.calls.append((method, params, timeout))
        if method == "thread/start":
            return {"thread": {"id": "thread"}}
        if method == "turn/start":
            if self.behavior == "success":
                self.notification("item/agentMessage/delta", turnId="other", delta="ignore")
                self.notification("item/agentMessage/delta", delta="Hello ")
                self.notification("item/agentMessage/delta", delta="canvas")
                self.notification("item/completed", item={"type": "agentMessage", "text": "Final"})
                self.notification("turn/completed", turn={"id": "old", "status": "completed"})
                self.notification("turn/completed", turn={"id": "turn", "status": "completed"})
            elif self.behavior == "failed":
                self.notification("turn/completed", turn={"id": "turn", "status": "failed"})
            elif self.behavior == "overflow":
                self.notification("item/agentMessage/delta", delta="x" * (bridge.MAX_TEXT + 1))
            elif self.behavior == "closed":
                self.events.put({"method": "bridge/closed"})
            return {"turn": {"id": "turn"}}
        if method == "turn/interrupt":
            return {}
        raise AssertionError(method)

    def close(self):
        self.calls.append(("close", {}, 0))
        self.closed = True
        self.process = None


class BridgeTests(unittest.TestCase):
    def setUp(self):
        FakeClient.instances = []
        FakeClient.signed_in = True
        FakeClient.behavior = "success"

    def serve(self, request=None, callback=None, timeout=1):
        incoming = Input(request or {"prompt": "Draw", "model": "", "context": {}})
        output = RecordingOutput(lambda e: callback(e, incoming) if callback else None)
        bridge.serve(incoming, output, FakeClient, timeout=timeout)
        return output.events, FakeClient.instances[-1], incoming

    def test_stream_and_contract(self):
        events, client, _ = self.serve()
        self.assertEqual(events[-1], {"event": "done", "ok": True,
                                     "text": "Final", "model": "mock-model"})
        self.assertEqual([e["text"] for e in events if e["event"] == "delta"],
                         ["Hello ", "canvas"])
        self.assertEqual(sum(e["event"] == "done" for e in events), 1)
        thread = next(p for m, p, _ in client.calls if m == "thread/start")
        turn = next(p for m, p, _ in client.calls if m == "turn/start")
        self.assertEqual(thread["dynamicTools"], bridge.TOOLS)
        self.assertEqual(thread["sandbox"], "read-only")
        self.assertEqual(turn["sandboxPolicy"], {"type": "readOnly"})
        self.assertEqual(thread["developerInstructions"], bridge.RULES)
        self.assertEqual(client.workspace, bridge.WORKSPACE)
        self.assertTrue(client.closed)

    def test_auth_and_model_errors(self):
        FakeClient.signed_in = False
        events, client, _ = self.serve()
        self.assertIn("codex login", events[-1]["error"])
        self.assertFalse(any(m == "turn/start" for m, _, _ in client.calls))
        FakeClient.signed_in = True
        events, client, _ = self.serve({"prompt": "Draw", "model": "unavailable"})
        self.assertIn("accessible", events[-1]["error"])
        self.assertFalse(any(m == "turn/start" for m, _, _ in client.calls))

    def test_cancel_during_turn_interrupts_before_close(self):
        FakeClient.behavior = "wait"
        def cancel(event, incoming):
            if event.get("text") == "Codex turn running":
                incoming.send({"cancel": True})
        events, client, _ = self.serve(callback=cancel)
        self.assertEqual(events[-1], {"event": "done", "ok": False, "error": "Cancelled"})
        methods = [m for m, _, _ in client.calls]
        self.assertLess(methods.index("turn/interrupt"), methods.index("close"))
        params = next(p for m, p, _ in client.calls if m == "turn/interrupt")
        self.assertEqual(params, {"threadId": "thread", "turnId": "turn"})

    def test_timeout_and_failure_close(self):
        for behavior in ("wait", "failed", "overflow", "closed"):
            with self.subTest(behavior=behavior):
                FakeClient.behavior = behavior
                events, client, _ = self.serve(timeout=0.15)
                self.assertFalse(events[-1]["ok"])
                self.assertTrue(client.closed)
                self.assertTrue(any(m == "turn/interrupt" for m, _, _ in client.calls))

    def test_cancel_before_start(self):
        incoming = Input({"prompt": "Draw"})
        incoming.send({"cancel": True})
        output = RecordingOutput()
        bridge.serve(incoming, output, FakeClient, timeout=1)
        self.assertFalse(output.events[-1]["ok"])
        self.assertTrue(FakeClient.instances[-1].closed)

    def test_client_is_closed_when_host_receives_done(self):
        for behavior in ("success", "failed", "overflow", "closed", "wait", "cancel", "auth"):
            with self.subTest(behavior=behavior):
                FakeClient.instances = []
                FakeClient.behavior = "wait" if behavior == "cancel" else behavior
                FakeClient.signed_in = behavior != "auth"
                received = []

                def receive(event, incoming):
                    if behavior == "cancel" and event.get("text") == "Codex turn running":
                        incoming.send({"cancel": True})
                    if event["event"] == "done":
                        client = FakeClient.instances[-1]
                        # This callback executes inside stdout.write: the host
                        # can kill Python now, before serve() returns.
                        self.assertTrue(client.closed)
                        self.assertIsNone(client.process)
                        self.assertEqual(client.calls[-1][0], "close")
                        received.append(event)

                events, _, _ = self.serve(callback=receive, timeout=0.15)
                self.assertEqual(received, [events[-1]])

    def test_invalid_initial_request_has_one_done(self):
        for value in ("[]\n", "{bad}\n", '{"prompt":""}\n',
                      '{"prompt":"x","context":[]}\n',
                      '{"prompt":"x","model":true}\n'):
            output = RecordingOutput()
            bridge.serve(io.StringIO(value), output, FakeClient)
            self.assertEqual(len(output.events), 1)
            self.assertFalse(output.events[0]["ok"])

    def transport(self, callback=None, timeout=0.5):
        incoming = Input()
        output = RecordingOutput(callback)
        host = bridge.HostTransport(incoming, bridge.Output(output), tool_timeout=timeout)
        reader = threading.Thread(target=host.read, daemon=True)
        reader.start()
        self.addCleanup(lambda: (host.finished.set(), incoming.lines.put(""), reader.join(1)))
        return host, incoming, output

    def test_tool_reply_routing_and_denial(self):
        host, incoming, output = self.transport()
        def reply(event):
            incoming.send({"id": 999, "ok": True, "result": "wrong"})
            incoming.send({"id": event["id"], "ok": False, "error": "User denied"})
        output.callback = reply
        result = host.tool("run_editor_command", {"command": "shape.rectangle", "params": {}})
        self.assertFalse(result["success"])
        self.assertEqual(result["contentItems"][0]["text"], "User denied")
        self.assertEqual(output.events[0], {"event": "tool", "id": 1,
            "name": "run_editor_command", "arguments": {"command": "shape.rectangle", "params": {}}})
        self.assertFalse(host.pending)

    def test_concurrent_replies_out_of_order(self):
        host, incoming, output = self.transport()
        emitted = queue.Queue()
        output.callback = emitted.put
        results = {}
        threads = [threading.Thread(target=lambda n=n: results.update({n: host.tool(n, {})}))
                   for n in ("inspect_editor", "inspect_canvas")]
        for thread in threads:
            thread.start()
        calls = [emitted.get(timeout=1), emitted.get(timeout=1)]
        for event in reversed(calls):
            incoming.send({"id": event["id"], "ok": False, "error": event["name"]})
        for thread in threads:
            thread.join(1)
            self.assertFalse(thread.is_alive())
        for name, result in results.items():
            self.assertEqual(result["contentItems"][0]["text"], name)

    def test_canvas_is_real_input_image(self):
        # Original synthetic 1x1 PNG, no external asset or live model needed.
        png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII="
        host, incoming, output = self.transport()
        output.callback = lambda e: incoming.send({"id": e["id"], "ok": True,
            "result": {"pngBase64": png, "width": 1, "height": 1}})
        result = host.tool("inspect_canvas", {})
        self.assertTrue(result["success"])
        self.assertEqual(result["contentItems"][0],
                         {"type": "inputImage", "imageUrl": "data:image/png;base64," + png})
        self.assertNotIn("pngBase64", result["contentItems"][1]["text"])
        self.assertTrue(base64.b64decode(png).startswith(b"\x89PNG"))
        for image in ("not base64", base64.b64encode(b"not png").decode()):
            with self.assertRaises(ValueError):
                bridge.tool_result("inspect_canvas", {"ok": True,
                    "result": {"pngBase64": image, "width": 1, "height": 1}})

    def test_cancellation_wakes_pending_tool(self):
        host, incoming, output = self.transport()
        output.callback = lambda _: incoming.send({"cancel": True})
        self.assertFalse(host.tool("inspect_editor", {})["success"])
        self.assertTrue(host.cancelled.is_set())
        self.assertFalse(host.pending)

    def test_tool_timeout_and_bad_reply(self):
        host, _, _ = self.transport(timeout=0.05)
        result = host.tool("inspect_editor", {})
        self.assertFalse(result["success"])
        self.assertIn("timed out", result["contentItems"][0]["text"])
        for reply in ({"id": True, "ok": True, "result": {}}, {"id": 1, "ok": "yes"},
                      {"id": 1, "ok": True}, {"id": 1, "ok": False}):
            host, incoming, output = self.transport()
            output.callback = lambda _, r=reply: incoming.send(r)
            self.assertFalse(host.tool("inspect_editor", {})["success"])
            self.assertTrue(host.cancelled.is_set())

    def test_tool_validation_and_budgets(self):
        host, _, output = self.transport()
        for name, args in (("shell", {}), ("inspect_editor", {"extra": 1}),
                           ("inspect_canvas", []), ("discover_commands", {"filter": 1}),
                           ("run_editor_command", {"command": "x", "params": []})):
            self.assertFalse(host.tool(name, args)["success"])
        self.assertEqual(output.events, [])
        with self.assertRaises(ValueError):
            bridge.parse_line("x" * (bridge.MAX_LINE + 1))
        with self.assertRaises(ValueError):
            bridge.validate_request({"prompt": "x", "context": {"x": "x" * bridge.MAX_TEXT}})
        stream = io.StringIO()
        emitter = bridge.Output(stream)
        emitter.emit("done", ok=False, error="test")
        emitter.emit("delta", text="late")
        self.assertEqual(len(stream.getvalue().splitlines()), 1)

    def test_raw_pipe_fragments_utf8_and_limits(self):
        pipe = bridge.PipeInput(123)
        data = (bridge.encode({"prompt": "画"}) + "\n" + bridge.encode({"cancel": True}) + "\n").encode()
        with patch.object(bridge.os, "read", side_effect=[data[:13], data[13:14], data[14:], b""]):
            self.assertEqual(bridge.parse_line(pipe.readline()), {"prompt": "画"})
            self.assertEqual(bridge.parse_line(pipe.readline()), {"cancel": True})
            self.assertEqual(pipe.readline(), "")
        pipe = bridge.PipeInput(123)
        with patch.object(bridge.os, "read", return_value=b"xxxxx"):
            with self.assertRaises(ValueError):
                pipe.readline(5)
        for line in ('{"number":NaN}', '{"number":1e999}'):
            with self.assertRaises(ValueError):
                bridge.parse_line(line)

    def test_inherited_kit_server_request_and_disabled_settings(self):
        # Import only; mock process creation and every RPC. No login/model call.
        kit = bridge.Path(r"D:\0\Noir Fiction Engine\documentation\toolkits\Codex_App_Bridge_Kit")
        if not kit.is_dir():
            self.skipTest("Supplied local kit unavailable")
        with patch.dict(bridge.os.environ, {"RAINEE_CODEX_KIT": str(kit)}):
            client_type = bridge.load_client()
        client = client_type.__new__(client_type)
        client.workspace = bridge.WORKSPACE
        client.executable = sys.executable
        client.start_lock = threading.Lock()
        client.process = None
        client.tools = bridge.TOOLS
        client.handler = lambda n, a: {"contentItems": [{"type": "inputText", "text": "denied"}], "success": False}
        sent = []
        client._send = sent.append
        client.rpc = lambda *a, **k: {}
        kit_module = __import__(client_type.__mro__[2].__module__, fromlist=["subprocess"])
        with patch.object(kit_module.subprocess, "Popen") as popen, patch.object(kit_module.threading, "Thread"):
            client.start()
            args = popen.call_args.args[0]
            self.assertEqual(args[0], sys.executable)
            for setting in ('features.shell_tool=false', 'features.unified_exec=false',
                            'features.apply_patch_freeform=false', 'features.apps=false',
                            'web_search="disabled"', 'sandbox_mode="read-only"',
                            'forced_login_method="chatgpt"', 'mcp_servers={}', 'plugins={}'):
                self.assertIn(setting, args)
            self.assertNotIn("OPENAI_API_KEY", popen.call_args.kwargs["env"])
        client._server_request({"id": 5, "method": "item/tool/call", "params": {
            "tool": "run_editor_command", "arguments": {"command": "shape.rectangle", "params": {}}}})
        self.assertFalse(sent[-1]["result"]["success"])
        client._server_request({"id": 6, "method": "item/commandExecution/requestApproval"})
        self.assertIn("error", sent[-1])


if __name__ == "__main__":
    unittest.main()
