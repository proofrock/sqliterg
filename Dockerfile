FROM alpine:latest as build

RUN apk add --no-cache rust cargo sqlite-libs sqlite-dev sed

COPY . /build
WORKDIR /build

RUN cp Cargo.toml Cargo.toml.orig
RUN sed 's/^rusqlite.*$/rusqlite = { version = "~0", features = ["serde_json", "load_extension"] }/' Cargo.toml.orig > Cargo.toml

RUN ["cargo", "build", "--release"]

# Now copy it into our base image.
FROM alpine:latest

COPY --from=build /build/target/release/sqliterg /

# TODO why libgcc?
RUN apk add --no-cache sqlite-libs libgcc

EXPOSE 12321
VOLUME /data

CMD ["/sqliterg"]
