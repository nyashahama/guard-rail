#!/usr/bin/env python3
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


class Handler(BaseHTTPRequestHandler):
    def do_POST(self):
        length = int(self.headers.get("content-length", "0"))
        body = self.rfile.read(length)
        try:
            payload = json.loads(body.decode("utf-8"))
        except json.JSONDecodeError:
            payload = {"raw": body.decode("utf-8", errors="replace")}

        print(json.dumps({"path": self.path, "payload": payload}), flush=True)
        response = json.dumps({"upstream": "ok", "received_event": payload.get("event")}).encode("utf-8")
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("x-upstream-version", "pilot-demo")
        self.send_header("content-length", str(len(response)))
        self.end_headers()
        self.wfile.write(response)

    def log_message(self, format, *args):
        return


if __name__ == "__main__":
    server = ThreadingHTTPServer(("127.0.0.1", 19090), Handler)
    print("pilot demo upstream listening on 127.0.0.1:19090", flush=True)
    server.serve_forever()
