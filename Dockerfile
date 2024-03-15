FROM rust:alpine

WORKDIR /workspace
RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.bfsu.edu.cn/g' /etc/apk/repositories \
    && apk add --no-cache git musl-dev

WORKDIR /workspace/qasmsim-agent
COPY . .
RUN cargo build --release

ENTRYPOINT [ "/bin/sh" ]