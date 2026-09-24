FROM debian:bookworm-slim
COPY target/release/aienos-waitlist /usr/local/bin/aienos-waitlist
USER 65532:65532
EXPOSE 18171
ENTRYPOINT ["/usr/local/bin/aienos-waitlist"]
