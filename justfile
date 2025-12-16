precommit:
    cargo fmt --all -- --check
    cargo clippy --fix --allow-dirty --allow-staged
    cargo machete
    cargo test

delete_db:
    rm "/Users/shaankhosla/Library/Application Support/repeat/cards.db"
