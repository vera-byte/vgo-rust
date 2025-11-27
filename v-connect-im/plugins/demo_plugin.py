#!/usr/bin/env python3
# 简易HTTP桥接插件示例 / Simple HTTP bridge plugin demo
# 依赖标准库，无需第三方包。运行前请确保 v-connect-im 已启用 HTTP Bridge 插件，
# 并且 HTTP API (默认 http://127.0.0.1:8080) 可以访问。

import http.server
import json
import threading
import time
import urllib.request
import urllib.error
from typing import Optional


PLUGIN_SERVER_HOST = "127.0.0.1"
PLUGIN_SERVER_PORT = 9102
VCON_API_BASE = "http://127.0.0.1:8080"


class PluginState:
    def __init__(self) -> None:
        self.plugin_id: Optional[str] = None
        self.token: Optional[str] = None

    def registered(self) -> bool:
        return self.plugin_id is not None and self.token is not None


STATE = PluginState()


def http_post(url: str, payload: dict):
    data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(
        url,
        data=data,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=5) as resp:
        body = resp.read().decode("utf-8")
        return json.loads(body) if body else {}


def register_plugin(callback_url: str):
    """注册新插件 / Register new plugin"""
    payload = {
        "name": "demo_python_plugin",
        "callback_url": callback_url,
        "capabilities": ["message", "room", "webhook", "connection", "user"],
    }
    resp = http_post(f"{VCON_API_BASE}/v1/plugins/register", payload)
    STATE.plugin_id = resp["plugin_id"]
    STATE.token = resp["token"]
    print(f"[plugin] registered: id={STATE.plugin_id} token={STATE.token}")


def reconnect_plugin(callback_url: str):
    """使用已有凭证重连 / Reconnect using existing credentials"""
    if not STATE.registered():
        return register_plugin(callback_url)
    payload = {
        "token": STATE.token,
        "callback_url": callback_url,
        "capabilities": ["message", "room", "webhook", "connection", "user"],
    }
    try:
        resp = http_post(
            f"{VCON_API_BASE}/v1/plugins/{STATE.plugin_id}/reconnect", payload
        )
        print(f"[plugin] reconnected: id={STATE.plugin_id} callback={callback_url}")
        return resp
    except urllib.error.HTTPError as e:
        if e.code == 400:
            print("[plugin] reconnect failed, re-registering...")
            return register_plugin(callback_url)
        raise


def send_heartbeat():
    while True:
        if STATE.registered():
            try:
                http_post(
                    f"{VCON_API_BASE}/v1/plugins/{STATE.plugin_id}/heartbeat",
                    {"token": STATE.token},
                )
            except urllib.error.URLError as e:
                print(f"[plugin] heartbeat failed: {e}")
        time.sleep(5)


def ack_event(event_id: str):
    if not STATE.registered():
        return
    try:
        http_post(
            f"{VCON_API_BASE}/v1/plugins/{STATE.plugin_id}/ack",
            {"token": STATE.token, "event_id": event_id},
        )
    except urllib.error.URLError as e:
        print(f"[plugin] ack failed: {e}")


class CallbackHandler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        raw = self.rfile.read(length)
        try:
            payload = json.loads(raw.decode("utf-8"))
        except json.JSONDecodeError:
            payload = {}

        event_type = payload.get("event_type")
        event_id = payload.get("event_id")
        print(f"[plugin] event={event_type} data={payload}")

        # 示例：处理各类事件 / Example: handle various events
        if event_type:
            if event_type.startswith("room."):
                print(f"[plugin] 🏠 room event: {event_type}")
            elif event_type.startswith("connection."):
                print(f"[plugin] 🔗 connection event: {event_type}")
            elif event_type.startswith("user."):
                print(f"[plugin] 👤 user event: {event_type}")
            elif event_type.startswith("message."):
                print(f"[plugin] 💬 message event: {event_type}")
            elif event_type.startswith("webhook."):
                print(f"[plugin] 🌐 webhook event: {event_type}")
            elif event_type.startswith("control."):
                print(f"[plugin] ⚙️  control event: {event_type}")

        # 回复200让服务器继续发送后续事件
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(json.dumps({"status": "ok"}).encode("utf-8"))

        # 发送ACK通知
        if event_id:
            ack_event(event_id)

    def log_message(self, fmt, *args):
        # 静默 http.server 默认日志
        return


def start_callback_server():
    attempts = [PLUGIN_SERVER_PORT, 0]
    last_error = None
    for port in attempts:
        try:
            server = http.server.ThreadingHTTPServer(
                (PLUGIN_SERVER_HOST, port), CallbackHandler
            )
            actual_port = server.server_address[1]
            thread = threading.Thread(
                target=server.serve_forever, daemon=True
            )
            thread.start()
            callback_url = (
                f"http://{PLUGIN_SERVER_HOST}:{actual_port}/callback"
            )
            print(
                f"[plugin] callback server listening on {callback_url} "
                f"(requested_port={port})"
            )
            return callback_url
        except PermissionError as exc:
            last_error = exc
            print(
                f"[plugin] port {port} permission denied, trying fallback..."
            )
        except OSError as exc:
            last_error = exc
            print(
                f"[plugin] port {port} unavailable ({exc}), trying fallback..."
            )
    raise SystemExit(f"unable to bind callback server: {last_error}")


def main():
    callback_url = start_callback_server()
    # 注册插件
    for _ in range(10):
        try:
            register_plugin(callback_url)
            break
        except urllib.error.URLError as e:
            print(f"[plugin] register failed: {e}, retrying...")
            time.sleep(3)
    else:
        raise SystemExit("register failed, abort")

    # 启动心跳线程
    heartbeat_thread = threading.Thread(target=send_heartbeat, daemon=True)
    heartbeat_thread.start()

    # 保持进程存活
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        print("[plugin] stopping plugin")
        if STATE.registered():
            try:
                http_post(
                    f"{VCON_API_BASE}/v1/plugins/{STATE.plugin_id}/stop",
                    {"token": STATE.token},
                )
            except urllib.error.URLError as e:
                print(f"[plugin] stop notification failed: {e}")


if __name__ == "__main__":
    main()

