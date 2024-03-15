FROM rust:alpine

WORKDIR /workspace
RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.bfsu.edu.cn/g' /etc/apk/repositories \
    && apk add --no-cache git musl-dev

WORKDIR /workspace/qasmsim-agent
COPY . .
RUN cargo build --release && mv target/release/qasmsim-agent /bin/qasmsim-agent \
    && cargo clean && rm -rf /usr/local/cargo \
    && rm -rf /usr/local/rustup

ENTRYPOINT [ "/bin/qasmsim-agent" ]