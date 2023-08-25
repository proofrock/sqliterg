#! /bin/bash

../target/release/sqliterg  2> /dev/null &

sleep 1

if ps aux | grep "sqliterg" | grep -v grep; then
    pkill -f "sqliterg"
    exit 1
else
    exit 0
fi