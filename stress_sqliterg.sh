#!/bin/bash

URL="http://localhost:12321/query"
CONTENT_TYPE="Content-Type: application/json"
REQ='{"transaction": [{"query": "SELECT * FROM TBL"},{"query": "SELECT * FROM TBL"}]}'
REQUESTS=1000

pkill -x ws4sqlite
pkill -x sqliterg

cargo build --release &> /dev/null
target/release/sqliterg --db test/bubbu.db &> /dev/null &

start_time=$(date +%s.%N)

for i in $(seq 1 $REQUESTS); do
  curl -s -X POST -H "$CONTENT_TYPE" -d "$REQ" -o /dev/null "$URL"
done;

end_time=$(date +%s.%N)

echo -n "Elapsed seconds in sqliterg: "
echo "$end_time - $start_time" | bc

pkill -x sqliterg
