# Build configuration
ARG project_name=binbot

# Set up rust build environment
FROM rust:1.66.1-alpine3.17 as build
ARG project_name
ENV USER=root
ENV RUSTFLAGS="-C target-cpu=native"

# Prepare build environment
RUN apk add --no-cache musl-dev
WORKDIR /usr/src

# Build the project
COPY . .
RUN touch ./src/main.rs && cargo build --release

# Create a minimal docker file with only the resulting binary
FROM scratch
ARG project_name
COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=build /usr/src/target/*/release/$project_name ./$project_name
USER 1000
CMD ["./$project_name"]
