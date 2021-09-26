FROM rust:1.55.0 as build
WORKDIR /app
COPY . /app
RUN cargo build --release

FROM gcr.io/distroless/static:nonroot
COPY --from=build --chown=nonroot:nonroot /app/target/release/kubernetes-version-exporter /
ENTRYPOINT ["./kubernetes-version-exporter"]
