#!/usr/bin/env python3
"""Local mock of the Plane developer API for release smoke.

Release smoke must exercise `plane api me` end to end without reaching the real
Plane deployment: production sits behind an IP allowlist that CI runners are not
part of, and we never want a real API token in CI. This server stands in for
`/api/v1/users/me/`. It also asserts the CLI sends the token as `X-API-Key`, so
the smoke covers the auth header without any real secret.
"""
from __future__ import annotations

import argparse
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

ME_PATH = "/api/v1/users/me/"
ME_USER = {
    "id": "smoke-user-id",
    "email": "smoke@plane.test",
    "display_name": "Plane Smoke",
    "first_name": "Plane",
    "last_name": "Smoke",
}


def build_handler(expected_key: str) -> type[BaseHTTPRequestHandler]:
    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_args) -> None:  # silence default request logging
            pass

        def _send_json(self, status: int, payload: dict[str, object]) -> None:
            body = json.dumps(payload).encode("utf-8")
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self) -> None:  # noqa: N802 - required handler name
            if self.path != ME_PATH:
                self._send_json(404, {"error": f"unexpected path: {self.path}"})
                return
            if self.headers.get("X-API-Key") != expected_key:
                self._send_json(401, {"error": "missing or invalid X-API-Key"})
                return
            self._send_json(200, ME_USER)

    return Handler


def main() -> None:
    parser = argparse.ArgumentParser(description="Mock Plane API for release smoke.")
    parser.add_argument("--port", required=True, type=int)
    parser.add_argument("--key", required=True)
    args = parser.parse_args()

    server = ThreadingHTTPServer(("127.0.0.1", args.port), build_handler(args.key))
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
