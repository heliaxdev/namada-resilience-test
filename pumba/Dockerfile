FROM alpine:3.22

ARG PUMBA_VERSION=0.11.6
ARG TARGETARCH
ENV PUMBA_VERSION=${PUMBA_VERSION}
ENV TARGETARCH=${TARGETARCH}

RUN apk add --no-cache bash curl docker-cli

RUN case "${TARGETARCH}" in \
      amd64) ARCH_SUFFIX="amd64" ;; \
      arm64) ARCH_SUFFIX="arm64" ;; \
      *) echo "Unsupported architecture: ${TARGETARCH}" && exit 1 ;; \
    esac && \
    curl -L "https://github.com/alexei-led/pumba/releases/download/${PUMBA_VERSION}/pumba_linux_${ARCH_SUFFIX}" \
      -o /usr/local/bin/pumba && \
    chmod +x /usr/local/bin/pumba

COPY entrypoint.sh /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
