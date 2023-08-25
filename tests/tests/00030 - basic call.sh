#! /bin/bash

../target/release/sqliterg  --db environment/test.db &

sleep 1

REQ='{"transaction":[{"query":"SELECT 1"}]}'

curl -H "Content-Type: application/json" -d $REQ -v http://localhost:12321/test/exec > environment/out 2> environment/err

if grep "< HTTP/1.1 200 OK" environment/err; then
    pkill -f "sqliterg"
    echo "Return code not OK"
    exit 1
fi

if [ "$(cat environment/out | jq '.results | length')" -ne "1" ]; then
    pkill -f "sqliterg"
    echo "Results number should be 1"
    exit 1
fi

if [ "$(cat environment/out | jq '.results.success')" -ne "true" ]; then
    pkill -f "sqliterg"
    echo "Results should be a success"
    exit 1
fi

pkill -f "sqliterg"
