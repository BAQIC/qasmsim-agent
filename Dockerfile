FROM rust:alpine

WORKDIR /workspace
RUN sed -i 's/dl-cdn.alpinelinux.org/mirrors.bfsu.edu.cn/g' /etc/apk/repositories \
    && apk add --no-cache git
RUN git clone https://github.com/BAQIC/qasmsim-agent.git

WORKDIR /workspace/qasmsim-agent

ENTRYPOINT [ "cargo", "run" ]