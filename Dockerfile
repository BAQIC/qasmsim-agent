FROM rust:alpine AS builder

WORKDIR /workspace
RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.bfsu.edu.cn/g' /etc/apk/repositories \
    && apk add --no-cache git musl-dev

WORKDIR /workspace/qasmsim-agent
COPY . .
RUN cargo build --release && mv target/release/qasmsim-agent /bin/qasmsim-agent \
    && cargo clean && rm -rf /usr/local/cargo \
    && rm -rf /usr/local/rustup

FROM alpine:latest
RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.bfsu.edu.cn/g' /etc/apk/repositories \
    && apk add musl --no-cache

COPY --from=builder /bin/qasmsim-agent /bin/qasmsim-agent

ENTRYPOINT [ "/bin/qasmsim-agent" ]