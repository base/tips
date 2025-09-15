# tips

### Notes
Run the whole stack locally:

ingress: `just db && sleep 3 && cargo run -p tips-ingress`

builder-playground: `go run main.go cook opstack --external-builder http://host.docker.internal:4444`
op-rbuilder: `rm -rf /Users/danyal/Library/Application\ Support/reth/ && just run-playground`
send-txns: `cd ..op-rbuilder && just send-txn`
connect db: `psql -d postgres://postgres:postgres@localhost:5432/postgres`