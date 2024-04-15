FROM ubuntu:23.10

WORKDIR /home/

COPY . .

RUN bash ./setup.sh

# confirm installation
RUN rustc --version
RUN cargo --version