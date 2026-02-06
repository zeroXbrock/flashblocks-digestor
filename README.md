# flashblocks-digestor

> *name subject to change*

A stream of MEV-related data from flashblocks.

## quickstart

```sh
# run digestor on (default) sepolia-base flashblocks stream
cargo run --bin flashblocks-digestor

# run digestor on some other flashblocks stream
cargo run --bin flashblocks-digestor -u $WS_FLASHBLOCKS_URL

# check out the other options
cargo run --bin flashblocks-digestor -- -h
```

## supported streams

- SSE (server-sent events): `cargo run --bin flashblocks-digestor -s sse`
- websockets: `cargo run --bin flashblocks-digestor -s websocket`

port for either protocol: `9001`

### sse

```sh
curl -L http://localhost:9001/events
```

### ws

```sh
cargo run --bin ws-subscriber
```
