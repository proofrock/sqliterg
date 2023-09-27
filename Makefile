.PHONY: test

profile:
	bash profiler/stress_sqliterg.sh
	bash profiler/stress_ws4sqlite.sh

test:
	- pkill sqliterg
	make build-debug
	cd tests; go test -v -timeout 5m

test-short:
	- pkill sqliterg
	make build-debug
	cd tests; go test -v -timeout 1m -short

build-debug:
	cargo build

build:
	cargo build --release

build-all:
	rm -rf bin
	- mkdir bin
	bash -c 'cross build --target `uname -m`-unknown-linux-musl --release'
	bash -c 'tar cjf bin/sqliterg-v0.0.0-`uname -m`-musl-bundled.tar.gz -C target/`uname -m`-unknown-linux-musl/release/ sqliterg'

update:
	cargo update
	cd tests && go get -u
	cd tests && go mod tidy

lint:
	cargo clippy 2> clippy_results.txt

docker:
	docker run --privileged --rm tonistiigi/binfmt --install arm64,arm
	docker buildx build --no-cache --platform linux/amd64 -t germanorizzo/sqliterg:v0.0.0-x86_64 --push .
	docker buildx build --no-cache --platform linux/arm/v7 -t germanorizzo/sqliterg:v0.0.0-arm --push .
	docker buildx build --no-cache --platform linux/arm64 -t germanorizzo/sqliterg:v0.0.0-aarch64 --push .
	- docker manifest rm germanorizzo/sqliterg:v0.0.0
	docker manifest create germanorizzo/sqliterg:v0.0.0 germanorizzo/sqliterg:v0.0.0-x86_64 germanorizzo/sqliterg:v0.0.0-arm germanorizzo/sqliterg:v0.0.0-aarch64
	docker manifest push germanorizzo/sqliterg:v0.0.0
	- docker manifest rm germanorizzo/sqliterg:latest
	docker manifest create germanorizzo/sqliterg:latest germanorizzo/sqliterg:v0.0.0-x86_64 germanorizzo/sqliterg:v0.0.0-arm germanorizzo/sqliterg:v0.0.0-aarch64
	docker manifest push germanorizzo/sqliterg:latest

docker-test-and-zbuild-all:
	- mkdir bin
	docker buildx build -f Dockerfile.binaries --target export -t tmp_binaries_build . --output bin