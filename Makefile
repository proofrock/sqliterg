.PHONY: test

clean:
	cargo clean
	rm -rf bin
	- docker image prune -af
	- docker builder prune -af
	rm -f profiler/*.class

update:
	cargo update
	cd tests && go get -u
	cd tests && go mod tidy

lint:
	cargo clippy 2> clippy_results.txt

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

build-static-nostatic:
	rm -rf bin
	- mkdir bin
	bash -c "RUSTFLAGS='-C target-feature=+crt-static' cargo build --release --target `uname -m`-unknown-linux-gnu"
	bash -c "tar czf bin/sqliterg-v0.18.0-linux-`uname -m`-static-bundled.tar.gz -C target/`uname -m`-unknown-linux-gnu/release/ sqliterg"
	cp Cargo.toml Cargo.toml.orig
	sed 's/^rusqlite.*$$/rusqlite = { version = "~0", features = [\"serde_json\", \"load_extension\"] }/' Cargo.toml.orig > Cargo.toml
	bash -c "cargo build --release --target `uname -m`-unknown-linux-gnu"
	bash -c "tar czf bin/sqliterg-v0.18.0-linux-`uname -m`-dynamic.tar.gz -C target/`uname -m`-unknown-linux-gnu/release/ sqliterg"
	mv Cargo.toml.orig Cargo.toml

build-macos:
	rm -rf bin
	- mkdir bin
	cargo build --release
	tar czf bin/sqliterg-v0.18.0-macos-x86_64-bundled.tar.gz -C target/release/ sqliterg
	cargo build --release --target aarch64-apple-darwin
	tar czf bin/sqliterg-v0.18.0-macos-aarch64-bundled.tar.gz -C target/aarch64-apple-darwin/release/ sqliterg
	cp Cargo.toml Cargo.toml.orig
	sed 's/^rusqlite.*$$/rusqlite = { version = "~0", features = [\"serde_json\", "load_extension"] }/' Cargo.toml.orig > Cargo.toml
	cargo build --release
	tar czf bin/sqliterg-v0.18.0-macos-x86_64-dynamic.tar.gz -C target/release/ sqliterg
	cargo build --release --target aarch64-apple-darwin
	tar czf bin/sqliterg-v0.18.0-macos-aarch64-dynamic.tar.gz -C target/aarch64-apple-darwin/release/ sqliterg	
	mv Cargo.toml.orig Cargo.toml

docker:
	docker run --privileged --rm tonistiigi/binfmt --install arm64
	docker buildx build --no-cache --platform linux/amd64 -t germanorizzo/sqliterg:v0.18.0-x86_64 --push .
	docker buildx build --no-cache --platform linux/arm64 -t germanorizzo/sqliterg:v0.18.0-aarch64 --push .
	- docker manifest rm germanorizzo/sqliterg:v0.18.0
	docker manifest create germanorizzo/sqliterg:v0.18.0 germanorizzo/sqliterg:v0.18.0-x86_64 germanorizzo/sqliterg:v0.18.0-aarch64
	docker manifest push germanorizzo/sqliterg:v0.18.0
	- docker manifest rm germanorizzo/sqliterg:latest
	docker manifest create germanorizzo/sqliterg:latest germanorizzo/sqliterg:v0.18.0-x86_64 germanorizzo/sqliterg:v0.18.0-aarch64
	docker manifest push germanorizzo/sqliterg:latest

docker-edge:
	# in Cargo.toml, set 'version = "0.x.999"' where x is the current minor
	docker run --privileged --rm tonistiigi/binfmt --install arm64,arm
	docker buildx build --no-cache --platform linux/amd64 -t germanorizzo/sqliterg:edge --push .

docker-zbuild-linux:
	- mkdir bin
	docker run --privileged --rm tonistiigi/binfmt --install arm64,arm
	docker buildx build --no-cache --platform linux/amd64 -f Dockerfile.binaries --target export -t tmp_binaries_build . --output bin
	docker buildx build --no-cache --platform linux/arm64 -f Dockerfile.binaries --target export -t tmp_binaries_build . --output bin
	# Doesn't work. armv7-unknown-linux-gnueabihf must be used. Anyway, for now ARMv7 is out of scope.
	# docker buildx build --no-cache --platform linux/arm/v7 -f Dockerfile.binaries --target export -t tmp_binaries_build . --output bin

