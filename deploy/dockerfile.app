FROM debian:forky

RUN apt update \
    && apt-get install --no-install-recommends --yes \
       ca-certificates \
    && apt-get clean
