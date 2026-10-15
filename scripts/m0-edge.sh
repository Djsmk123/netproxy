#!/bin/zsh
# M0 edge cases: TLS failure classes, timeouts/slow/cancel, WebSocket. Usage: scripts/m0-edge.sh [debug|release]
ROOT=${0:A:h:h}; B=$ROOT/target/${1:-debug}; W=$(mktemp -d); cd $W
$B/np-harness --port 9443 --ca-out up-ca.pem 2>h.err & H=$!
sleep 1
$B/np-spike-rust --port 8899 --ca-out px-ca.pem --upstream-ca up-ca.pem >events.jsonl 2>p.err & P=$!
sleep 2
X=(-s -x http://127.0.0.1:8899 --cacert px-ca.pem -o /dev/null -w '%{http_code}')
echo "== TLS/connect failures (expect 502 + class)"
for u in 9444 9445 9446 9999; do printf ":%s -> " $u; curl $X https://localhost:$u/small; echo; done
echo "== slow drip 2s (expect 200, ~2s)"; curl $X -w ' %{time_total}s\n' "https://localhost:9443/slow?ms=2000"
echo "== client cancel at 0.5s of 5s body"; curl $X --max-time 0.5 "https://localhost:9443/slow?ms=5000"; echo
echo "== websocket echo x50"; python3 -I $ROOT/scripts/ws_client.py 8899 px-ca.pem localhost 9443 50
sleep 1; kill -INT $P; sleep 1; kill $H 2>/dev/null
python3 -I - <<'PY'
import json,collections
ev=[json.loads(l) for l in open('events.jsonl')]
print("failure classes:", dict(collections.Counter(e['class'] for e in ev if e['ev']=='flow_failed')))
print("body_aborted:", [(e['dir'],e['total']) for e in ev if e['ev']=='body_aborted'])
ws=[e for e in ev if e['ev']=='ws_message']
print("ws_messages:", collections.Counter(e['dir'] for e in ws))
PY
tail -1 p.err; if [ -n "$KEEP" ]; then echo "workdir kept: $W"; else rm -rf "$W"; fi
