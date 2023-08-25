#! /bin/bash

../target/release/sqliterg  --db environment/test.db &

sleep 1

if ! ps aux | grep "sqliterg" | grep -v grep; then
    pkill -f "sqliterg"
    exit 1
fi

if ! ls environment/test.db; then
    pkill -f "sqliterg"
    exit 1
fi

pkill -f "sqliterg"
