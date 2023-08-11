#!/bin/bash

URL="http://localhost:12321/bubbu"
CONTENT_TYPE="Content-Type: application/json"
REQ='{"transaction":[{"statement":"DELETE FROM TBL"},{"query":"SELECT * FROM TBL"},{"statement":"INSERT INTO TBL (ID, VAL) VALUES (:id, :val)","values":{"id":0,"val":"zero"}},{"statement":"INSERT INTO TBL (ID, VAL) VALUES (:id, :val)","valuesBatch":[{"id":1,"val":"uno"},{"id":2,"val":"due"}]},{"noFail":true,"statement":"INSERT INTO TBL (ID, VAL) VALUES (:id, :val, 1)","valuesBatch":[{"id":1,"val":"uno"},{"id":2,"val":"due"}]},{"statement":"INSERT INTO TBL (ID, VAL) VALUES (:id, :val)","valuesBatch":[{"id":3,"val":"tre"}]},{"query":"SELECT * FROM TBL WHERE ID=:id","values":{"id":1}},{"statement":"DELETE FROM TBL"}]}'
REQUESTS=10000

pkill -x ws4sqlite
pkill -x sqliterg

wget -q https://github.com/proofrock/ws4sqlite/releases/download/v0.15.0/ws4sqlite-v0.15.0-linux-arm64.tar.gz
tar xzf ws4sqlite-v0.15.0-linux-arm64.tar.gz &> /dev/null
rm -f ws4sqlite-v0.15.0-linux-arm64.tar.gz

./ws4sqlite --db test/bubbu.db &> /dev/null &

sleep 1

start_time=$(date +%s.%N)

for i in $(seq 1 $REQUESTS); do
  curl -s -X POST -H "$CONTENT_TYPE" -d "$REQ" -o /dev/null "$URL"
done;

end_time=$(date +%s.%N)

echo -n "Elapsed seconds in ws4sqlite: "
echo "$end_time - $start_time" | bc

pkill -x ws4sqlite
rm -f ws4sqlite