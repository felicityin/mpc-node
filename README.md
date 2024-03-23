# Description

Client for MPC protocol.

> This repo is inspired by [tss-lib](https://github.com/ququzone/tss-lib), [cggmp21](https://github.com/dfns/cggmp21) and [mpc-over-signal](https://github.com/ZenGo-X/mpc-over-signal).

# Run

Server
```
git clone https://github.com/felicityin/mpc-signal.git
cd mpc-signal
cargo run
```

Client 1
```
cargo build

./target/debug/tss-cli keygen --server-url ws://127.0.0.1:8080 --room c05554ae-b4ee-4976-ac05-97aaf3c98a23 -i 0 -t 2 -n 2 output_1
```

Client 2
```
./target/debug/tss-cli keygen --server-url ws://127.0.0.1:8080 --room c05554ae-b4ee-4976-ac05-97aaf3c98a23 -i 1 -t 2 -n 2 output_2
```
