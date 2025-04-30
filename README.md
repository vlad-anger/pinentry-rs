
#### Test pinentry TUI locally

- Make sure to have [just](https://github.com/casey/just) installed.
- Clone repo, build it.
- Configure pinentry in `$HOME/.gnupg/gpg-agent.conf`

Add entry:
`pinentry-program <REPO_PATH>/target/debug/pinentry-tui-rs`

- In project root invoke

```sh
just test-sign
```

