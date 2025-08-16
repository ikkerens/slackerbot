# Build arguments
ARG PROJECT_NAME=slackerbot
ARG BUILD_TYPE=debug

FROM rust:1.89.0-alpine3.22 AS base_debug
ENV RUSTFLAGS=""
ARG RELEASE_MODE=""

FROM base_debug AS base_instrumented
ONBUILD ENV RUSTFLAGS="${RUSTFLAGS} -Cprofile-generate=/pgo"
ONBUILD ARG RELEASE_MODE="--release"

FROM base_debug AS base_with_profile
ARG PROJECT_NAME
ONBUILD COPY $PROJECT_NAME.profdata /pgo/$PROJECT_NAME.profdata
ONBUILD ENV RUSTFLAGS="${RUSTFLAGS} -Cprofile-use=/pgo/$PROJECT_NAME.profdata"
ONBUILD ARG RELEASE_MODE="--release"

FROM base_debug AS base_release
ONBUILD ARG RELEASE_MODE="--release"

# Set up rust build environment
FROM base_${BUILD_TYPE} AS build
ARG PROJECT_NAME
ENV USER=root

# Prepare the static lib required for musl compilation
RUN apk add --no-cache musl-dev

# Create a skeleton workspace
WORKDIR /usr/src
COPY . .
RUN cargo build --package $PROJECT_NAME --bin $PROJECT_NAME $RELEASE_MODE

# Create a minimal docker file with only the resulting binary
FROM scratch
ARG PROJECT_NAME
COPY --from=build /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=build /usr/src/web ./web
COPY --from=build /usr/src/target/*/$PROJECT_NAME ./app
USER 1000
CMD ["./app"]