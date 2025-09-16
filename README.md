# tips
A prototype of a Transaction Inclusion Pipeline Service for private sequencers that does not use the P2P mempool. The project
aims to increase throughput, improve transaction tracking, reduce latency and add support for bundles.

This project is currently at:

https://github.com/flashbots/builder-playground
https://github.com/base/tips/pull/new/prototype
https://github.com/base/op-rbuilder/pull/new/tips-prototype


### Local Development
You can run the whole system locally with:

tips:
```sh
just db && sleep 3 && just ingress
```

builder-playground:
```sh
# TODO: Figure out the flashblocks/websocket proxy/validator setup
go run main.go cook opstack --external-builder http://host.docker.internal:4444 --enable-latest-fork 0
```

op-rbuilder:
```sh
just run-playground

# Send transactions with
just send-txn
```

[optional]  tips
```sh
just ui
```

Debugging notes:
```sh
# Connect to the database
psql -d postgres://postgres:postgres@localhost:5432/postgres

# Update the UI's schema
just ui-db-schema
```