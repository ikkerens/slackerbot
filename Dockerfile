# Build configuration
ARG project_name=slackerbot

# Set up rust build environment
FROM rust:1.69.0-alpine3.17 as build
ARG project_name
ENV USER=root

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
COPY --from=build /usr/src/web ./web
COPY --from=build /usr/src/target/*/$project_name ./app
USER 1000
CMD ["./app"]
