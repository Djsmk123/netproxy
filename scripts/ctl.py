#!/usr/bin/env python3 -I
"""Tiny netproxyd control client: ctl.py <data-dir> <method> [json-params]   (prints the result as JSON)."""
import json, os, socket, sys
data, method = sys.argv[1], sys.argv[2]
params = json.loads(sys.argv[3]) if len(sys.argv) > 3 else None
run = os.path.join(data, "run")
token = open(os.path.join(run, "control.token")).read().strip()
ep = os.path.join(run, "control.endpoint")
sock_path = open(ep).read().strip() if os.path.exists(ep) else os.path.join(run, "control.sock")
s = socket.socket(socket.AF_UNIX); s.connect(sock_path); f = s.makefile("rw", buffering=1)
def call(i, m, p):
    f.write(json.dumps({"jsonrpc": "2.0", "id": i, "method": m, "params": p}) + "\n")
    while True:
        r = json.loads(f.readline())
        if r.get("id") == i: return r
call(1, "hello", {"token": token, "api_version": 1, "client": {"name": "ctl.py"}})
r = call(2, method, params)
print(json.dumps(r.get("result", r.get("error")), indent=2))
