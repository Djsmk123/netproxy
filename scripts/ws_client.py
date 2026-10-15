"""Minimal WebSocket echo check through the proxy (stdlib only): CONNECT -> TLS -> upgrade -> N echoes."""
import base64, os, socket, ssl, struct, sys
proxy_port, ca, host, port, n = int(sys.argv[1]), sys.argv[2], sys.argv[3], int(sys.argv[4]), int(sys.argv[5])
if proxy_port:  # 0 = connect directly (isolates proxy vs upstream)
    s = socket.create_connection(("127.0.0.1", proxy_port), timeout=5)
    s.sendall(f"CONNECT {host}:{port} HTTP/1.1\r\nHost: {host}:{port}\r\n\r\n".encode())
    assert b"200" in s.recv(4096).split(b"\r\n")[0], "CONNECT failed"
else:
    s = socket.create_connection(("127.0.0.1", port), timeout=5)
ctx = ssl.create_default_context(cafile=ca)
t = ctx.wrap_socket(s, server_hostname=host)
key = base64.b64encode(os.urandom(16)).decode()
t.sendall(f"GET /ws HTTP/1.1\r\nHost: {host}:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n".encode())
head = b""
while b"\r\n\r\n" not in head:
    c = t.recv(1)
    if not c: raise EOFError(f"closed during handshake: {head!r}")
    head += c
assert b"101" in head.split(b"\r\n")[0], head
def send(payload: bytes):
    mask = os.urandom(4)
    hdr = bytes([0x81, 0x80 | len(payload)]) + mask
    t.sendall(hdr + bytes(b ^ mask[i % 4] for i, b in enumerate(payload)))
def recv_exact(k):
    b = b""
    while len(b) < k:
        c = t.recv(k - len(b))
        if not c: raise EOFError("connection closed")
        b += c
    return b
ok = 0
for i in range(n):
    msg = f"msg-{i}".encode(); send(msg)
    h = recv_exact(2); ln = h[1] & 0x7F
    if recv_exact(ln) == msg: ok += 1
print(f"ws_echo_ok={ok}/{n}")
sys.exit(0 if ok == n else 1)
