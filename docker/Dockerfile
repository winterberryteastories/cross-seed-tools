FROM rust:1.84.1-bullseye AS build

# create a new empty shell project
RUN USER=root cargo new --bin cross-seed-tools
WORKDIR /cross-seed-tools

# copy over your manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# this build step will cache your dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy your source tree
COPY ./src ./src

# build for release
RUN rm ./target/release/deps/cross_seed_tools*
RUN cargo build --release

# our final base
FROM rust:1.84.1-slim-bullseye

# copy the build artifact from the build stage
COPY --from=build /cross-seed-tools/target/release/cross-seed-tools .

# set the startup command to run your binary
CMD ["./cross-seed-tools"]
