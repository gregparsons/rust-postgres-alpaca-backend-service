web:
	cargo run --release -- frontend_ui
webdev:
	cargo run -- frontend_ui
frontend:
	./frontend/all.sh
backend:
	make -B docker_build_backend
	./backend/all.sh
sqlx:
	# https://lib.rs/crates/sqlx-cli (is out of date about --workspace)
	# 0.6.3
	cargo sqlx prepare --merged
	# 0.7.0
	#cargo sqlx prepare --workspace
maintenance:
	cargo fmt
	cargo clippy
	cargo audit
	cargo test
	# rustup update
	# rustup self update
	# cargo doc
test:
	export CONFIG_LOCATION=test;cargo test -p trader
devport:
	ssh -L 54320:10.1.1.205:54320 swimr205

docker_build_frontend:
	docker build --file dockerfile_frontend --tag frontend .
docker_build_backend:
	docker build --file dockerfile_backend --tag backend .
docker_build_backend_rest:
	docker build --file dockerfile_backend_rest --tag backend_rest .
