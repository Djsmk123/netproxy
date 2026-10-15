#!/bin/zsh
# M0 smoke: runs harness + spike proxy, drives curl, verifies the event stream vs ground truth.
# Usage: scripts/m0-smoke.sh [profile: debug|release]
set -u
ROOT=${0:A:h:h}; PROFILE=${1:-debug}; B=$ROOT/target/$PROFILE
W=$(mktemp -d); cd $W
$B/np-harness --port 9443 --ca-out up-ca.pem 2>h.err & H=$!
sleep 1
$B/np-spike-rust --port 8899 --ca-out px-ca.pem --upstream-ca up-ca.pem >events.jsonl 2>p.err & P=$!
sleep 2
X=(-s -x http://127.0.0.1:8899 --cacert px-ca.pem)
U=https://localhost:9443
curl $X --http1.1 $U/small >/dev/null
curl $X --http2 $U/small >/dev/null
curl $X -d hello $U/echo >/dev/null
curl $X $U/__np_mock >/dev/null
EXP=$(curl -s --cacert up-ca.pem -D - -o /dev/null "$U/big?mb=100" | grep -i x-np-sha256 | tr -d '\r' | awk '{print $2}')
curl $X "$U/big?mb=100" -o /dev/null
curl $X $U/reset -o /dev/null
for i in $(seq 1 100); do echo "url = $U/small"; echo "output = /dev/null"; done > cfg.txt
curl $X --http2 --parallel --parallel-max 100 -K cfg.txt >/dev/null
sleep 1
kill -INT $P; sleep 1; kill $H 2>/dev/null
python3 -I - "$EXP" <<'PY'
import json,sys,collections
exp=sys.argv[1]; ev=[json.loads(l) for l in open('events.jsonl')]
by=collections.defaultdict(list)
for e in ev:
    if 'flow' in e: by[e['flow']].append(e)
def kinds(f): return [e['ev'] for e in by[f]]
req=[f for f in by if 'request_headers' in kinds(f)]
conn=[f for f in by if 'connection_opened' in kinds(f)]
resp_ok=[f for f in req if 'response_headers' in kinds(f)]
done=[f for f in req if 'flow_completed' in kinds(f)]
bad=[f for f in req if kinds(f).count('response_headers')>1]
big=[e for e in ev if e['ev']=='body_end' and e['dir']=='res' and e['total']==100*1024*1024]
errs=[e for e in ev if e['ev']=='body_error']
mocks=[e for e in ev if e['ev']=='mock_hit']
print(f"flows={len(req)} connections={len(conn)} with_response={len(resp_ok)} completed={len(done)} duplicate_responses={len(bad)}")
print(f"100MB sha256 {'MATCH' if big and big[0]['sha256']==exp else 'MISMATCH'}")
print(f"body_errors(reset)={len(errs)} mock_hits={len(mocks)}")
print(open('p.err').read().strip().splitlines()[-1])
PY
if [ -n "$KEEP" ]; then echo "workdir kept: $W"; else rm -rf "$W"; fi
