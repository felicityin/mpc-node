# Description

This repo is inspired by [tss-lib](https://github.com/ququzone/tss-lib).

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

./target/debug/tss-cli keygen --server-url ws://127.0.0.1:8080 --room c05554ae-b4ee-4976-ac05-97aaf3c98a23 -i 1 -t 1 -n 2 output
```

Client 2
```
./target/debug/tss-cli keygen --server-url ws://127.0.0.1:8080 --room c05554ae-b4ee-4976-ac05-97aaf3c98a23 -i 1 -t 1 -n 2 output
```
