[private]
just:
    just -l

tty:
    cargo run --bin pinentry-tty-rs

tui:
    cargo run --bin pinentry-tui-rs

# Reloads gpg agent
[group("gpg/utils")]
gpg-reload-agent:
    gpgconf --kill gpg-agent

[group("test-utils")]
test-sign:
    @just gpg-reload-agent
    @rm -f justfile.asc
    gpg --sign -a -vv justfile
