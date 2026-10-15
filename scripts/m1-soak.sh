#!/bin/zsh
# Full-pipeline soak: netproxyd (engine + sequencer + SQLite) under mixed load. Usage: m1-soak.sh [secs]
ROOT=${0:A:h:h}; B=$ROOT/target/release; D=${1:-120}; W=$(mktemp -d); cd $W
$B/np-harness --port 9443 --ca-out up-ca.pem 2>h.err & H=$!
sleep 1
$B/netproxyd --data-dir $W/data --listen 127.0.0.1:8899 --upstream-ca up-ca.pem --max-db-mb ${MAX_DB_MB:-2048} --autostart 2>d.err & P=$!
sleep 2
cp $W/data/ca/ca.pem px-ca.pem
echo "== latency (engine + store enabled)"; $B/np-bench --n 5000 2>&1 | sed 's/(ms).*ADDED/ ... ADDED/'
echo "== soak ${D}s"
$B/np-bench --soak $D --par ${PAR:-64} > bench.out 2>&1 & C=$!
echo "t_s rss_mb db_mb" > rss.txt; t=0
while kill -0 $C 2>/dev/null; do echo "$t $(( $(ps -o rss= -p $P) / 1024 )) $(du -sm $W/data/netproxy.db* 2>/dev/null | awk '{s+=$1} END{print s}')" >> rss.txt; sleep 10; t=$((t+10)); done
cat bench.out
python3 -I $ROOT/scripts/ctl.py $W/data engine.status
python3 -I $ROOT/scripts/ctl.py $W/data storage.status
python3 -I $ROOT/scripts/ctl.py $W/data runs.list | grep -E "events|flows"
cat rss.txt | awk 'NR==1 || NR%3==0'
echo "bodies: $(find $W/data/bodies -type f 2>/dev/null | wc -l | tr -d " ") files, $(du -sm $W/data/bodies 2>/dev/null | cut -f1) MB (cap 1024 MB)"
kill -INT $P; sleep 3; kill $H
echo "daemon stderr:"; tail -3 d.err; if [ -n "$KEEP" ]; then echo "workdir kept: $W"; else rm -rf "$W"; fi
