#!/bin/zsh
# 10-minute (default) sustained mixed load via the proxy; samples proxy RSS every 10 s. Usage: m0-soak.sh [secs]
ROOT=${0:A:h:h}; B=$ROOT/target/release; D=${1:-600}; W=$(mktemp -d); cd $W
$B/np-harness --port 9443 --ca-out up-ca.pem 2>h.err & H=$!
sleep 1; $B/np-spike-rust --port 8899 --ca-out px-ca.pem --upstream-ca up-ca.pem >events.jsonl 2>p.err & P=$!
sleep 2
$B/np-bench --soak $D > bench.out 2>&1 & C=$!
echo "t_s rss_mb" > rss.txt
t=0; while kill -0 $C 2>/dev/null; do echo "$t $(( $(ps -o rss= -p $P) / 1024 ))" >> rss.txt; sleep 10; t=$((t+10)); done
kill -INT $P; sleep 1; kill $H
cat bench.out; tail -1 p.err; echo "events: $(wc -l < events.jsonl) lines, $(du -m events.jsonl | cut -f1) MB"; cat rss.txt
if [ -n "$KEEP" ]; then echo "workdir kept: $W"; else rm -rf "$W"; fi
