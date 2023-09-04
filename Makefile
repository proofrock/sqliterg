.PHONY: test

profile:
	bash profiler/stress_sqliterg.sh
	bash profiler/stress_ws4sqlite.sh

test:
	make build-debug
	cd tests; go test -v -timeout 6m

build-debug:
	cargo build

build:
	cargo build --release

update:
	cargo update
	cd tests && go get -u
	cd tests && go mod tidy
