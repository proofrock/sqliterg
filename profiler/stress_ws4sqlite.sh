#!/bin/bash

URL="http://localhost:12321/test_ws4sqlite"
REQUESTS=20000

cd "$(dirname "$0")"

rm -f environment/*.db*
rm -f ws4sqlite*

pkill -x ws4sqlite
pkill -x sqliterg

wget -q https://github.com/proofrock/ws4sqlite/releases/download/v0.15.1/ws4sqlite-v0.15.1-linux-amd64.tar.gz
tar xzf ws4sqlite-v0.15.0-linux-amd64.tar.gz &> /dev/null

./ws4sqlite --db environment/test_ws4sqlite.db &

javac Profile.java

sleep 1

echo -n "Elapsed seconds in ws4sqlite: "
java -cp ./ Profile $REQUESTS $URL $REQ

rm Profile.class

pkill -x ws4sqlite
rm -f ws4sqlite*
rm -f environment/*.db*
