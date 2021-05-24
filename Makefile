build: compile assemble

client:
	cd client && yarn && yarn run start:dev
.PHONY: client

server:
	cd server && RUST_BACKTRACE=1 RUST_LOG=debug cargo run
.PHONY: server

server-reload:
	cd server && RUST_LOG=debug systemfd --no-pid -s http::8002 -- cargo watch -x run
.PHONY: server-reload

extracti18n:
	cd client && cargo i18n

compile:
	cd client && cargo i18n && yarn && yarn run build && yarn run css
	cd server && cargo build --release
.PHONY: compile

assemble:
	rm -rf dist
	mkdir dist
	cp -R client/dist/ dist/public
	cp target/release/thevalley_server dist/
.PHONY: assemble
