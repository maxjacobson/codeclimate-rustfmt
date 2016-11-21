FROM jimmycuadra/rust:1.13.0

WORKDIR /usr/src/app

RUN cargo install rustfmt --vers 0.6.3 --root /usr/local

COPY Cargo.toml /usr/src/app/
COPY Cargo.lock /usrc/src/app/

COPY . /usr/src/app
RUN cargo install --root /usr/local

RUN adduser -u 9000 app
RUN chown -R app:app /usr/src/app

USER app

VOLUME /code
WORKDIR /code

# CMD ["cargo fmt -- --write-mode checkstyle"]
CMD ["/usr/local/bin/codeclimate-rustfmt"]
