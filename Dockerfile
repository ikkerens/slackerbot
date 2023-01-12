# Build configuration
ARG project_name=binbot

# Set up rust build environment
FROM rust:1.66.1-alpine3.17 as build
ENV RUSTFLAGS="-C target-cpu=native"
ARG project_name

# Create layer for the dependencies, so we don't have to rebuild them later
WORKDIR /usr/src
RUN USER=root cargo new $project_name

# Build the actual source
COPY . .
RUN touch ./src/main.rs && cargo build --release

# Create a minimal docker file with only the resulting binary
FROM scratch
ARG project_name
COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=build /usr/src/target/*/release/$project_name ./app
USER 1000
CMD ["./app"]
