use std::{
    fs::{File, OpenOptions},
    os::fd::{AsRawFd, FromRawFd, RawFd},
};

mod error;
pub mod handler;

// pub type AssuanHandler = fn(&mut AssuanContext, String);

// pub struct AssuanCmd {
//     pub name: &'static str,
//     pub handler: AssuanHandler,
//     pub help: Option<&'static str>,
// }

// impl AssuanCmd {
//     pub fn from_vec_tuples(items: Vec<(&'static str, AssuanHandler)>) -> Vec<Self> {
//         items.into_iter().map(|v| v.into()).collect::<Vec<Self>>()
//     }
// }

// impl From<(&'static str, AssuanHandler)> for AssuanCmd {
//     fn from(value: (&'static str, AssuanHandler)) -> Self {
//         AssuanCmd {
//             name: value.0,
//             handler: value.1,
//             help: None,
//         }
//     }
// }

pub(crate) struct Inbound {
    fd: File,
}

pub(crate) struct Outbound {
    fd: File,
}

pub struct AssuanContext {
    pub(crate) inbound: Inbound,
    pub(crate) outbound: Outbound,

    pub commands: Vec<String>,
    pub options: Vec<String>,
}

impl AssuanContext {
    pub fn new(fd_in: impl AsRawFd, fd_out: impl AsRawFd) -> Self {
        logc();
        let fd_in = unsafe { File::from_raw_fd(fd_in.as_raw_fd()) };
        let fd_out = unsafe { File::from_raw_fd(fd_out.as_raw_fd()) };

        Self {
            inbound: Inbound { fd: fd_in },
            outbound: Outbound { fd: fd_out },
            commands: vec![],
            options: vec![],
        }
    }

    pub fn register_commands(mut self, commands: &[&str]) -> Self {
        self.commands
            .append(&mut commands.iter().map(|v| v.to_string()).collect());
        self
    }

    pub fn register_options(mut self, options: &[&str]) -> Self {
        self.options
            .append(&mut options.iter().map(|v| v.to_string()).collect());
        self
    }
}

pub fn logc() {
    use std::io::Write;

    // std::fs::OpenOptions::new()
    //     .write(true)
    //     .truncate(true)
    //     // .append(true)
    //     .open("/home/rin/git/pinentry-rs/log.txt")
    //     .unwrap()
    //     .write_all("".as_bytes())
    //     .ok();
}

pub fn logw(line: &str) {
    use std::io::Write;

    // std::fs::OpenOptions::new()
    //     .append(true)
    //     .open("/home/rin/git/pinentry-rs/log.txt")
    //     .unwrap()
    //     .write_all(format!("{line}\n").as_bytes())
    //     .ok();
}
