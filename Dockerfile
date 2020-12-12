FROM rust as planner
WORKDIR app
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM rust as cacher
WORKDIR app
RUN cargo install cargo-chef
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM rust:latest as builder

# add wasm32 target, install latest version of trunk, install v0.2.69 of wasm-bindgen-cli
RUN rustup target add wasm32-unknown-unknown && \
    TRUNK_VERSION=$(curl -s https://api.github.com/repos/thedodd/trunk/releases/latest | grep -oP '(?<="tag_name": ")[^"]*') && \
    wget -qO- https://github.com/thedodd/trunk/releases/download/${TRUNK_VERSION}/trunk-x86_64-unknown-linux-gnu.tar.gz | tar -xzf- && \
    mv trunk /usr/bin && \
    wget -qO- https://github.com/rustwasm/wasm-bindgen/releases/download/0.2.69/wasm-bindgen-0.2.69-x86_64-unknown-linux-musl.tar.gz | tar -xzf- && \
    mv wasm-bindgen-0.2.69-x86_64-unknown-linux-musl/wasm-bindgen /usr/bin

WORKDIR /app

COPY . .
COPY --from=cacher /app/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo

RUN trunk --config frontend/Trunk.toml build frontend/index.html --release --dist /app/dist
RUN cargo build -p backend --release


FROM gcr.io/distroless/cc
COPY --from=builder /app/target/release/backend /
COPY --from=builder /app/dist /static
ENV DIST_DIR="/static"
CMD ["./backend"]

# sudo docker run --network host --name waichu --env "DATABASE_URL=postgresql://waichu:password@localhost:5432/waichu" --env "DIST_DIR=/static" waichu
