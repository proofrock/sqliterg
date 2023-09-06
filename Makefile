.PHONY: test

profile:
	bash profiler/stress_sqliterg.sh
	bash profiler/stress_ws4sqlite.sh

test:
	- pkill sqliterg
	make build-debug
	cd tests; go test -v -timeout 10m

test-short:
	- pkill sqliterg
	make build-debug
	cd tests; go test -v -timeout 1m -short

build-debug:
	cargo build

build:
	cargo build --release

update:
	cargo update
	cd tests && go get -u
	cd tests && go mod tidy

lint:
	cargo clippy 2> clippy_results.txt