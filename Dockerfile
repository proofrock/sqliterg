# FROM rust:latest as build
# 
# WORKDIR /app
# COPY . .
# 
# RUN cargo build
# RUN pwd
# RUN ls -al target/debug
# --release

FROM alpine:latest as build

RUN apk add --no-cache rust cargo sqlite-libs sqlite-dev

COPY . /build
WORKDIR /build
RUN ["cargo", "build", "--release"]

# Now copy it into our base image.
FROM alpine:latest

COPY --from=build /build/target/release/sqliterg /

# TODO why libgcc?
RUN apk add --no-cache sqlite-libs libgcc

EXPOSE 12321
VOLUME /data

CMD ["/sqliterg"]