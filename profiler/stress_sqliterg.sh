#!/bin/bash

cd "$(dirname "$0")"

URL="http://localhost:12321/test"
REQUESTS=20000

mkdir environment/backup
rm -f environment/*.db*

pkill -x ws4sqlite
pkill -x sqliterg

cd ..
cargo build --release
cd profiler
../target/release/sqliterg --db environment/test.db &

javac Profile.java

sleep 1

echo -n "Elapsed seconds in sqliterg: "
java -cp ./ Profile $REQUESTS $URL $REQ

rm Profile.class

pkill -x sqliterg
rm -f environment/*.db*
