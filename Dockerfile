# syntax=docker/dockerfile:1.7-labs

FROM rust:trixie as prefetch
WORKDIR /src/wazuh-per-hit-alert
RUN apt update && apt install -y libssl-dev && apt clean
COPY Cargo.toml /src/wazuh-per-hit-alert/
RUN mkdir /src/wazuh-per-hit-alert/src
RUN echo 'fn main() {println!("stub!");}' >/src/wazuh-per-hit-alert/src/main.rs
RUN cargo b --release

FROM prefetch as build
WORKDIR /src/wazuh-per-hit-alert
COPY Cargo.toml /src/wazuh-per-hit-alert/
COPY src /src/wazuh-per-hit-alert/src
RUN touch src/main.rs && cargo b --release --verbose && cp target/*/wazuh-per-hit-alert .

FROM debian:trixie as wazuh-per-hit-alert
RUN apt update && apt install -y ca-certificates libssl3 && apt clean
RUN mkdir -p /etc/wazuh-per-hit-alert 
COPY --from=build /src/wazuh-per-hit-alert/wazuh-per-hit-alert /bin/wazuh-per-hit-alert
COPY config.toml /etc/wazuh-per-hit-alert/
VOLUME /etc/wazuh-per-hit-alert
WORKDIR /tmp
ENTRYPOINT [ "/bin/wazuh-per-hit-alert", "/etc/wazuh-per-hit-alert/config.toml" ]
