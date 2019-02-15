FROM rust:1.32.0 as builder

RUN set -ex && \
    apt-get update && \
    apt-get --no-install-recommends --yes install \
    clang \
    libclang-dev \
    llvm-dev \
    libncurses5 \
    libncursesw5 \
    cmake \
    git

# create a new empty shell project
RUN USER=root cargo new --bin grinbox
WORKDIR /grinbox

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
COPY ./grinboxlib ./grinboxlib

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy your source tree
COPY ./src ./src

# build for release
RUN rm ./target/release/deps/grinbox*
RUN cargo build --release

# runtime stage
FROM debian:9.4

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y locales openssl curl

RUN sed -i -e 's/# en_US.UTF-8 UTF-8/en_US.UTF-8 UTF-8/' /etc/locale.gen && \
    dpkg-reconfigure --frontend=noninteractive locales && \
    update-locale LANG=en_US.UTF-8

ENV LANG en_US.UTF-8

RUN adduser --disabled-login --home /grinbox --gecos "" grinbox

USER grinbox

COPY --from=builder ./grinbox/target/release/grinbox /grinbox/grinbox

WORKDIR /grinbox

COPY ./start.sh ./start.sh

CMD ["./start.sh"]

EXPOSE 13420