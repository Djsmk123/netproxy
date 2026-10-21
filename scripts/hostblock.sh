#!/usr/bin/env bash
# Block ad/telemetry domains system-wide via /etc/hosts.
#   sudo ./hostblock.sh enable   # add (or refresh) the managed block
#   sudo ./hostblock.sh disable  # remove the managed block
#   ./hostblock.sh status        # show whether blocked
# Only the section between the BEGIN/END markers is ever touched.
# Note: /etc/hosts has no wildcards, so each hostname is listed explicitly.
set -euo pipefail

HOSTS=/etc/hosts
BEGIN="# BEGIN hostblock"
END="# END hostblock"

DOMAINS=(
  hesads.akamaized.net
  advertising.amazon.dev
  mads.amazon.com
  mads-eu.amazon.com
  mads-na.amazon.com
  mads-fe.amazon.com
  unagi-na.amazon.com
  unagi-eu.amazon.com
  global.telemetry.insights.video.a2z.com
  prod.us-east-1.sonar.prime-video.amazon.dev
  aviary.amazon.com
  # Prime Video filter list v3.6 (exact hostnames only; wildcards/regex can't live in /etc/hosts)
  s0s7.api.amazonvideo.com
  web-checks.a2z.com
  p-v-logs.amazon.com
  zoar.triggers-v1.prod.mobile.weblab.a2z.com
  emt-fastly.live.pv-cdn.net
  s.amazon-adsystem.com
  mads.amazon-adsystem.com
  aiv-delivery.net
  a2hmcwiq4hwfgz-ats.iot.us-east-1.amazonaws.com
  device-metrics-us.amazon.com
  fls-na.amazon.com
  # getVideoAds endpoint host (/cdp/getVideoAds); blocks the whole host, see note in README/chat
  qgmg.api.amazonvideo.com
)

need_root() {
  [[ $EUID -eq 0 ]] || { echo "Run with sudo: sudo $0 $1" >&2; exit 1; }
}

strip_block() {
  # print /etc/hosts without the managed block
  awk -v b="$BEGIN" -v e="$END" '
    $0==b {skip=1; next}
    $0==e {skip=0; next}
    !skip {print}
  ' "$HOSTS"
}

flush_dns() {
  dscacheutil -flushcache 2>/dev/null || true
  killall -HUP mDNSResponder 2>/dev/null || true
}

write_hosts() {
  local tmp
  tmp=$(mktemp)
  cat > "$tmp"
  cp -p "$HOSTS" "$HOSTS.hostblock.bak"
  cat "$tmp" > "$HOSTS"   # preserve inode/perms
  rm -f "$tmp"
  flush_dns
}

case "${1:-}" in
  enable)
    need_root enable
    {
      strip_block
      echo "$BEGIN"
      for d in "${DOMAINS[@]}"; do
        echo "0.0.0.0 $d"
        echo "::      $d"
      done
      echo "$END"
    } | write_hosts
    echo "Blocked ${#DOMAINS[@]} domains."
    ;;
  disable)
    need_root disable
    strip_block | write_hosts
    echo "Removed hostblock entries."
    ;;
  status)
    if grep -qxF "$BEGIN" "$HOSTS"; then
      echo "ENABLED"
      sed -n "/^$BEGIN\$/,/^$END\$/p" "$HOSTS" | grep -v '^#' | grep -v '^::' | awk '{print "  " $2}'
    else
      echo "DISABLED"
    fi
    ;;
  *)
    echo "Usage: $0 {enable|disable|status}" >&2
    exit 2
    ;;
esac
