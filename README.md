# Wordle Archive

Personal Wordle archive.

## Deployment

### Things in common

* Set up the PostgreSQL schema using `db/schema.pgsql`.
* Fill the database table `sites` as needed.

### Without Docker

Possible on up-to-date instances of Arch Linux.

* Make sure you have Rust and Cargo installed.
* Build the application using `cargo build --release --all-targets`.
* Place `target/release/wordle-archive` and `contrib/sample-config.toml` into a directory of your choice; rename `sample-config.toml` to `config.toml`.
* Edit `config.toml` to your needs.
* Run `wordle-archive`.
* Point your browser to [localhost:8084/wordle-archive/](http://localhost:8084/wordle-archive/).
* Refer to `contrib/wordle-archive.service` for a Systemd unit file.

### Using Docker

Necessary on Debian Stable.

* Make sure you have Docker installed.
* Build the docker image using `docker build . -t wordle-archive`.
* Copy `contrib/sample-config.toml` to an otherwise empty directory (the below example uses `/etc/wordle-archive`) and rename it to `config.toml`.
* Edit `config.toml` to your needs. Set the `listen_addr` to `0.0.0.0:8084` and the database host to `host.docker.internal`.
* Run the docker image using:

```
docker run
    -v /etc/wordle-archive/:/config/
    -p 127.0.0.1:8084:8084
    --add-host=host.docker.internal:host-gateway
    --name wordle-archive
    wordle-archive
```

* Make sure that PostgreSQL accepts connections from that container.
* Point your browser to [localhost:8084/wordle-archive/](http://localhost:8084/wordle-archive/).

