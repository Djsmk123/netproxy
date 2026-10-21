#!/bin/bash
TOKEN=$1
METHOD=$2
PARAMS=${3:-"{}"}

(
  echo '{"jsonrpc":"2.0","id":1,"method":"'$METHOD'","params":'$PARAMS'}'
  sleep 0.5
) | nc -U ./test-data/run/control.sock
