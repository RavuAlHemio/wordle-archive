FROM archlinux:base-devel as build
RUN pacman -Syu --noconfirm
RUN pacman -S --needed --noconfirm cargo

COPY ./ /opt/wordle-archive
WORKDIR /opt/wordle-archive
RUN cargo build --release --all-targets
RUN cargo test --release


FROM archlinux:base
RUN pacman -Syu --noconfirm

RUN groupadd wordle && useradd -rm -d /opt/wordle-archive -g wordle wordle
USER wordle

COPY --from=0 /opt/wordle-archive/target/release/wordle-archive /opt/wordle-archive
RUN ln -s /config/config.toml /opt/wordle-archive/config.toml

WORKDIR /opt/wordle-archive
CMD ./wordle-archive
