save:
    savepoint -f rs cargo t

test:
    cargo t

docs:
    cargo doc

alias sd := serve-docs
serve-docs:
    cargo server --path ./target/doc

watch:
    watchexec "just docs & just test"

alias i := install
install:
    cargo install --path ./crates/waybar

alias wi := watch-install
watch-install:
    watchexec "just install"
