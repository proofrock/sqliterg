#!/bin/bash

URL="http://localhost:12321/db/bubbu"
REQUESTS=20000

rm -rf test/*.db*
sqlite3 test/bubbu.db "CREATE TABLE TBL (ID INT, VAL TEXT)"

pkill -x ws4sqlite
pkill -x sqliterg

cargo build --release
target/release/sqliterg --db test/bubbu.db &

cd profiler
javac Profile.java

sleep 1

echo -n "Elapsed seconds in sqliterg: "
java -cp ./ Profile $REQUESTS $URL $REQ

rm Profile.class
cd ..

pkill -x sqliterg
rm -rf test/*.db*
