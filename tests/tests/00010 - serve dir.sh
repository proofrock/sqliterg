#! /bin/bash

../target/release/sqliterg  --serve-dir . &

sleep 1

if curl http://localhost:12321/test.sh -s ; then
    pkill -f "sqliterg"
    exit 0
else
    pkill -f "sqliterg"
    exit 1
fi