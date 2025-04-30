use std::{io::Write, os::fd::AsRawFd};

use anyhow::anyhow;
use libpinentry::{Pinentry, PinentryResolver};
use termion::{input::TermRead, raw::IntoRawMode};

struct PinentryTty {}

impl PinentryResolver for PinentryTty {
    fn get_pin(&mut self, pinentry: &mut Pinentry) -> Result<String, libpinentry::error::Error> {
        let (mut fin, mut fout) = pinentry.get_resolver_fd_in_out()?;

        fout.write_all(b"password: ")
            .map_err(|err| anyhow!("Fail write to stdout {err}"))?;
        fout.flush()?;

        let _raw = fout
            .into_raw_mode()
            .map_err(|err| anyhow!("Fail transform fd raw mode {err}"))?;

        let pass = fin.read_line();

        let Ok(Some(pass)) = pass else {
            return Err(anyhow!("Fail read pass"));
        };

        Ok(pass)
    }
}

fn main() {
    let resolver = PinentryTty {};

    let fd_in = std::io::stdin().as_raw_fd();
    let fd_out = std::io::stdout().as_raw_fd();

    Pinentry::default().run_loop(fd_in, fd_out, resolver).ok();
}
