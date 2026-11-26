#!/usr/bin/env python3
from __future__ import annotations

import base64
from contextlib import ExitStack
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import mimetypes
import os
from pathlib import Path
import shutil
import socket
import struct
import subprocess
import threading
import sys
import time
from urllib.parse import urlsplit
from urllib.request import Request, urlopen

from processes import terminate

PAGE = r'''<!doctype html><meta charset="utf-8">
<link rel="icon" href="data:,">
<title>Installed Anbani browser compatibility proof</title>
<h1>Installed UMD and WebAssembly</h1><pre id="art"></pre>
<script src="/js/anbani.js"></script><script src="/textart/TextArt.js"></script>
<script type="module">
window.archiveResult = (async () => {
  const same = (actual, expected, label) => {
    if (actual !== expected) throw Error(label + ': '+JSON.stringify(actual)+' != '+JSON.stringify(expected));
    return actual;
  };
  const js = window.anbani, art = window['anbani-textart'];
  if (!js || !art) throw Error('missing actual UMD globals');
  const wasm = await import('/wasm/index.js');
  await wasm.load(); // default URL fetches the actual .wasm over HTTP
  const converted = same(js.core.convert('ანბანი','mkhedruli','mtavruli'), 'ᲐᲜᲑᲐᲜᲘ', 'UMD conversion');
  same(wasm.core.convert('ანბანი','mkhedruli','mtavruli'), converted, 'wasm typed conversion');
  same(wasm.call('core.convert','ანბანი','mkhedruli','mtavruli'), converted, 'wasm variadic call');
  same(js.nlp.contractions.expand('ა. შ.'), 'ასე შემდეგ', 'UMD contractions');
  same(js.nlp.contractions.contract('ასე შემდეგ'), 'ა.შ.', 'UMD contraction reverse');
  same(js.nlp.georgianisation.georgianise('kartuli ena','fast'), 'კარტული ენა', 'UMD fast mode');
  let rejected = false;
  try { js.nlp.georgianisation.georgianise('kartuli ena','balanced'); }
  catch { rejected = true; }
  if (!rejected) throw Error('UMD balanced mode must reject, not load source data');
  if (!(js.data.regex.mkhedruli instanceof RegExp)) throw Error('UMD regex lost its type');
  const rendered = same(art.generate('Anbani Block Small','ა'), ' ▀▙ \n  █ \n█▄█ \n', 'Block Small');
  document.querySelector('#art').textContent = rendered;
  await document.fonts.ready;
  await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  return {converted, wasm:wasm.core.convert('ანბანი','mkhedruli','mtavruli'), rendered,
          expanded:js.nlp.contractions.expand('ა. შ.'), fast:js.nlp.georgianisation.georgianise('kartuli ena','fast'), balancedRejected:rejected};
})();
// Store rejection immediately, so a module/setup failure is never a silent pass.
window.archiveResult.catch(error => { document.querySelector('#art').textContent = String(error.stack || error); });
</script>'''

class DevTools:

    def __init__(self, url):
        parsed = urlsplit(url)
        if parsed.scheme != "ws" or parsed.hostname not in ("127.0.0.1", "localhost"):
            raise ValueError("DevTools endpoint must be local ws")
        self.socket = socket.create_connection((parsed.hostname, parsed.port), timeout=10)
        self.socket.settimeout(65)
        self.buffer = bytearray()
        self.counter = 0
        self.events = []
        key = base64.b64encode(os.urandom(16)).decode("ascii")
        request = (f"GET {parsed.path} HTTP/1.1\r\nHost: {parsed.hostname}:{parsed.port}\r\n"
                   f"Upgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key}\r\n"
                   "Sec-WebSocket-Version: 13\r\n\r\n")
        self.socket.sendall(request.encode("ascii"))
        header = bytearray()
        while b"\r\n\r\n" not in header:
            data = self.socket.recv(4096)
            if not data:
                raise RuntimeError("Chrome closed websocket handshake")
            header.extend(data)
            if len(header) > 65536:
                raise RuntimeError("oversized websocket handshake")
        headers, remainder = bytes(header).split(b"\r\n\r\n", 1)
        expected = base64.b64encode(hashlib.sha1((key + "258EAFA5-E914-47DA-95CA-C5AB0DC85B11").encode("ascii")).digest()).decode("ascii")
        if not headers.startswith(b"HTTP/1.1 101 ") or f"sec-websocket-accept: {expected}".lower() not in headers.decode("ascii").lower():
            raise RuntimeError("invalid Chrome websocket handshake")
        self.buffer.extend(remainder)

    def read(self, length):
        while len(self.buffer) < length:
            data = self.socket.recv(max(4096, length - len(self.buffer)))
            if not data:
                raise RuntimeError("Chrome closed DevTools socket")
            self.buffer.extend(data)
        value = bytes(self.buffer[:length])
        del self.buffer[:length]
        return value

    def send(self, payload, opcode=1):
        mask = os.urandom(4)
        size = len(payload)
        header = bytes([0x80 | opcode])
        if size < 126:
            header += bytes([0x80 | size])
        elif size <= 65535:
            header += bytes([0x80 | 126]) + struct.pack("!H", size)
        else:
            header += bytes([0x80 | 127]) + struct.pack("!Q", size)
        self.socket.sendall(header + mask + bytes(value ^ mask[index % 4] for index, value in enumerate(payload)))

    def message(self):
        fragments = bytearray()
        while True:
            first, second = self.read(2)
            opcode = first & 15
            size = second & 127
            if size == 126:
                size = struct.unpack("!H", self.read(2))[0]
            elif size == 127:
                size = struct.unpack("!Q", self.read(8))[0]
            if size > 32 * 1024 * 1024:
                raise RuntimeError("oversized Chrome message")
            mask = self.read(4) if second & 0x80 else None
            payload = self.read(size)
            if mask:
                payload = bytes(value ^ mask[index % 4] for index, value in enumerate(payload))
            if opcode == 8:
                raise RuntimeError("Chrome closed DevTools session")
            if opcode == 9:
                self.send(payload, opcode=10)
                continue
            if opcode == 10:
                continue
            if opcode not in (0, 1):
                raise RuntimeError(f"unexpected websocket opcode {opcode}")
            fragments.extend(payload)
            if first & 0x80:
                return json.loads(fragments)

    def call(self, method, params=None):
        self.counter += 1
        identifier = self.counter
        self.send(json.dumps({"id": identifier, "method": method, "params": params or {}}).encode("utf-8"))
        deadline = time.monotonic() + 65
        while time.monotonic() < deadline:
            message = self.message()
            if message.get("id") == identifier:
                if "error" in message:
                    raise RuntimeError(f"{method}: {message['error']}")
                return message.get("result", {})
            if "method" in message:
                self.events.append(message)
        raise TimeoutError(f"DevTools {method} timed out")

    def wait_event(self, method, timeout=30):
        deadline = time.monotonic() + timeout
        try:
            while time.monotonic() < deadline:
                for index, event in enumerate(self.events):
                    if event.get("method") == method:
                        return self.events.pop(index)
                self.socket.settimeout(max(0.1, deadline - time.monotonic()))
                event = self.message()
                if "method" in event:
                    self.events.append(event)
            raise TimeoutError(f"Chrome event {method} timed out")
        finally:
            self.socket.settimeout(65)

    def close(self):
        self.socket.close()

def chrome_executable(explicit):
    candidate = explicit or os.environ.get("CHROME_BIN")
    if candidate:
        executable = shutil.which(candidate)
        if executable is None:
            raise ValueError(f"Chrome executable unavailable: {candidate}")
        return executable
    for name in ("google-chrome", "google-chrome-stable", "chromium", "chromium-browser"):
        if executable := shutil.which(name):
            return executable
    mac = Path("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome")
    if mac.is_file():
        return str(mac)
    raise ValueError("real Chrome is required; set CHROME_BIN or --browser")

def smoke(modules, temporary, artifacts, executable=None):
    executable = chrome_executable(executable)
    temporary.mkdir()
    routes = {"/": PAGE.encode("utf-8")}
    for route, installed in (("/js/anbani.js", modules / "anbani/dist/anbani.js"),
                             ("/textart/TextArt.js", modules / "anbani-textart/dist/TextArt.js")):
        routes[route] = installed.read_bytes()
    for name in ("index.js", "anbani_wasm.js", "anbani_wasm_bg.wasm"):
        routes["/wasm/" + name] = (modules / "@anbani/wasm" / name).read_bytes()
    requests = []
    forbidden = []

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            route = urlsplit(self.path).path
            requests.append(route)
            if route not in routes:
                forbidden.append(self.path)
                self.send_error(404)
                return
            payload = routes[route]
            self.send_response(200)
            mime = "text/html; charset=utf-8" if route == "/" else mimetypes.guess_type(route)[0] or "application/octet-stream"
            self.send_header("Content-Type", mime)
            self.send_header("Content-Length", str(len(payload)))
            self.send_header("Cache-Control", "no-store")
            self.end_headers()
            self.wfile.write(payload)

        def log_message(self, *args):
            pass

    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    server.daemon_threads = True
    server.timeout = 5
    thread = threading.Thread(target=server.serve_forever, kwargs={"poll_interval": 0.1}, daemon=True)
    thread.start()
    browser = None
    client = None
    log = (temporary / "chrome.log").open("wb")
    try:
        profile = temporary / "profile"
        browser = subprocess.Popen([executable, "--headless=new", "--no-sandbox", "--disable-dev-shm-usage",
                                    "--disable-background-networking", "--no-first-run", "--no-default-browser-check",
                                    "--window-size=1000,700", "--remote-debugging-port=0",
                                    f"--user-data-dir={profile}", "about:blank"], stdout=log, stderr=log,
                                   start_new_session=os.name != "nt",
                                   creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0)
        active = profile / "DevToolsActivePort"
        deadline = time.monotonic() + 30
        while not active.is_file():
            if browser.poll() is not None:
                raise RuntimeError("Chrome exited before DevTools became ready")
            if time.monotonic() >= deadline:
                raise TimeoutError("Chrome DevTools did not start in 30 seconds")
            time.sleep(0.1)
        port = int(active.read_text(encoding="utf-8").splitlines()[0])
        request = Request(f"http://127.0.0.1:{port}/json/new?about:blank", method="PUT")
        with urlopen(request, timeout=10) as response:
            tab = json.load(response)
        client = DevTools(tab["webSocketDebuggerUrl"])
        client.call("Page.enable")
        client.call("Runtime.enable")
        client.events.clear()
        client.call("Page.navigate", {"url": f"http://127.0.0.1:{server.server_port}/"})
        client.wait_event("Page.loadEventFired")
        evaluation = client.call("Runtime.evaluate", {
            "expression": """(async () => {
              const deadline = Date.now()+30000;
              while (!window.archiveResult) {
                if (Date.now()>deadline) throw Error('archive browser module did not initialize');
                await new Promise(resolve=>setTimeout(resolve,50));
              }
              return await Promise.race([window.archiveResult,
                new Promise((_, reject)=>setTimeout(()=>reject(Error('archive browser proof timed out')),30000))]);
            })()""",
            "awaitPromise": True, "returnByValue": True, "timeout": 60000,
        })
        if "exceptionDetails" in evaluation or "value" not in evaluation.get("result", {}):
            raise RuntimeError(f"real browser assertion failed: {evaluation}")
        result = evaluation["result"]["value"]
        expected_routes = set(routes)
        if forbidden or not expected_routes.issubset(set(requests)):
            raise RuntimeError(f"browser did not use only installed distribution routes: requested={requests}, forbidden={forbidden}")
        screenshot = base64.b64decode(client.call("Page.captureScreenshot", {"format": "png", "captureBeyondViewport": False})["data"], validate=True)
        if not screenshot.startswith(b"\x89PNG\r\n\x1a\n"):
            raise RuntimeError("Chrome did not produce a PNG screenshot")
        (artifacts / "browser-block-small.png").write_bytes(screenshot)
        proof = {"browser": executable, "results": result, "requests": requests, "served": sorted(routes),
                 "screenshot": "browser-block-small.png"}
        (artifacts / "browser-proof.json").write_text(json.dumps(proof, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(proof, ensure_ascii=False, indent=2), flush=True)
        with urlopen(f"http://127.0.0.1:{port}/json/close/{tab['id']}", timeout=10) as response:
            if response.status != 200:
                raise RuntimeError("Chrome refused to close the proof tab")
    finally:
        failure = sys.exception()
        try:
            with ExitStack() as cleanup:
                cleanup.callback(thread.join, timeout=5)
                cleanup.callback(server.server_close)
                cleanup.callback(server.shutdown)
                cleanup.callback(log.close)
                if browser is not None:
                    cleanup.callback(terminate, browser)
                if client is not None:
                    cleanup.callback(client.close)
            if thread.is_alive():
                raise RuntimeError("browser proof HTTP server did not stop")
        except BaseException:
            if failure is None:
                raise
