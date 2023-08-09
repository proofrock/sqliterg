#!/bin/bash

URL="http://localhost:12321/bubbu"
CONTENT_TYPE="Content-Type: application/json"
REQ='{"transaction": [{"query": "SELECT * FROM TBL"},{"query": "SELECT * FROM TBL"}]}'
REQUESTS=1000

pkill -x ws4sqlite
pkill -x sqliterg

wget -q https://github.com/proofrock/ws4sqlite/releases/download/v0.15.0/ws4sqlite-v0.15.0-linux-arm64.tar.gz
tar xzf ws4sqlite-v0.15.0-linux-arm64.tar.gz &> /dev/null
rm -f ws4sqlite-v0.15.0-linux-arm64.tar.gz

./ws4sqlite --db test/bubbu.db &> /dev/null &

start_time=$(date +%s.%N)

for i in $(seq 1 $REQUESTS); do
  curl -s -X POST -H "$CONTENT_TYPE" -d "$REQ" -o /dev/null "$URL"
done;

end_time=$(date +%s.%N)

echo -n "Elapsed seconds in ws4sqlite: "
echo "$end_time - $start_time" | bc

pkill -x ws4sqlite
rm -f ws4sqlite