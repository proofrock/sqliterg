#!/bin/bash

URL="http://localhost:12321/test_ws4sqlite"
REQUESTS=20000

rm -rf test/*.db*

pkill -x ws4sqlite
pkill -x sqliterg

wget -q https://github.com/proofrock/ws4sqlite/releases/download/v0.15.0/ws4sqlite-v0.15.0-linux-arm64.tar.gz
tar xzf ws4sqlite-v0.15.0-linux-arm64.tar.gz &> /dev/null
rm -f ws4sqlite-v0.15.0-linux-arm64.tar.gz

./ws4sqlite --db test/test_ws4sqlite.db &

cd profiler
javac Profile.java

sleep 1

echo -n "Elapsed seconds in ws4sqlite: "
java -cp ./ Profile $REQUESTS $URL $REQ

rm Profile.class
cd ..

pkill -x ws4sqlite
rm -f ws4sqlite
rm -rf test/*.db*
