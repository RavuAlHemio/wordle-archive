FROM archlinux:base-devel as build
RUN pacman -Syu --noconfirm
RUN pacman -S --needed --noconfirm cargo git python

COPY ./ /opt/wordle-archive
WORKDIR /opt/wordle-archive
RUN python3 cicd/version_stamp.py
RUN cargo build --release --all-targets
RUN cargo test --release


FROM archlinux:base
RUN pacman -Syu --noconfirm

RUN groupadd wordle && useradd -rm -d /opt/wordle-archive -g wordle wordle
USER wordle

COPY --from=0 /opt/wordle-archive/target/release/wordle-archive /opt/wordle-archive
RUN ln -s /config/config.toml /opt/wordle-archive/config.toml

HEALTHCHECK --interval=5m --timeout=10s CMD curl http://localhost:8084/ || exit 1

WORKDIR /opt/wordle-archive
CMD ./wordle-archive

