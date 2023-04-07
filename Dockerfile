FROM rust:1.68.0 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Final layer to minimize size
FROM gcr.io/distroless/cc-debian11
COPY --from=builder /app/target/release/beacon-metrics-gazer /beacon-metrics-gazer
ENTRYPOINT ["/beacon-metrics-gazer"]
