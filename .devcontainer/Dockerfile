FROM ubuntu:23.10

WORKDIR /home/

COPY . .

RUN bash ./setup.sh

# replace shell with bash so we can source files
RUN rm /bin/sh && ln -s /bin/bash /bin/sh

# confirm installation
RUN rustc --version
RUN cargo --version