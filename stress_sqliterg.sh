#!/bin/bash

URL="http://localhost:12321/db/test"
REQUESTS=20000

rm -rf test/*.db*

pkill -x ws4sqlite
pkill -x sqliterg

cargo build --release
target/release/sqliterg --db test/test.db &

cd profiler
javac Profile.java

sleep 1

echo -n "Elapsed seconds in sqliterg: "
java -cp ./ Profile $REQUESTS $URL $REQ

rm Profile.class
cd ..

pkill -x sqliterg
rm -rf test/*.db*
