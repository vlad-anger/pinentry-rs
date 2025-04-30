use std::{
    fs::File,
    io::BufReader,
    os::fd::{AsRawFd, FromRawFd, RawFd},
    str::FromStr,
};

pub mod error;
use anyhow::anyhow;
use error::Error;
use libassuan::{
    handler::{assuan_process, assuan_process_done, assuan_write_line, AssuanProcessResult},
    AssuanContext,
};

pub(crate) fn unenc(val: &str) -> Result<String, Error> {
    urlencoding::decode(val)
        .map(|v| v.to_string())
        .map_err(|err| anyhow!("Fail urlencoding::decode. {err}"))
}

const PE_CMD_SETDESC: &str = "SETDESC";
const PE_CMD_PROMPT: &str = "SETPROMPT";
const PE_CMD_KEYINFO: &str = "SETKEYINFO";
const PE_CMD_REPEAT: &str = "SETREPEAT";
const PE_CMD_SETOK: &str = "SETOK";
const PE_CMD_SETNOTOK: &str = "SETNOTOK";
const PE_CMD_SETCANCEL: &str = "SETCANCEL";
const PE_CMD_GETPIN: &str = "GETPIN";
const PE_CMD_SETTITLE: &str = "SETTITLE";
const PE_CMD_SETERROR: &str = "SETERROR";

const PINENTRY_COMMANDS: [&str; 10] = [
    PE_CMD_SETDESC,
    PE_CMD_PROMPT,
    PE_CMD_KEYINFO,
    PE_CMD_REPEAT,
    PE_CMD_SETOK,
    PE_CMD_SETNOTOK,
    PE_CMD_SETCANCEL,
    PE_CMD_GETPIN,
    PE_CMD_SETTITLE,
    PE_CMD_SETERROR,
];

#[derive(Debug, PartialEq, Eq)]
enum PinentryCmd {
    SetDesc,
    Prompt,
    KeyInfo,
    Repeat,
    SetOk,
    SetNotOk,
    SetCancel,
    GetPin,
    SetTitle,
    SetError,
}

impl FromStr for PinentryCmd {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            PE_CMD_SETDESC => Ok(Self::SetDesc),
            PE_CMD_PROMPT => Ok(Self::Prompt),
            PE_CMD_KEYINFO => Ok(Self::KeyInfo),
            PE_CMD_REPEAT => Ok(Self::Repeat),
            PE_CMD_SETOK => Ok(Self::SetOk),
            PE_CMD_SETNOTOK => Ok(Self::SetNotOk),
            PE_CMD_SETCANCEL => Ok(Self::SetCancel),
            PE_CMD_GETPIN => Ok(Self::GetPin),
            PE_CMD_SETTITLE => Ok(Self::SetTitle),
            PE_CMD_SETERROR => Ok(Self::SetError),
            _ => Err(anyhow!("{s} not valid PinentryCmd")),
        }
    }
}

const PE_OPT_TTYNAME: &str = "ttyname";

const PINENTRY_OPTIONS: [&str; 1] = [PE_OPT_TTYNAME];

#[derive(Debug, PartialEq, Eq)]
enum PinentryOption {
    TtyName,
}

impl FromStr for PinentryOption {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            PE_OPT_TTYNAME => Ok(Self::TtyName),
            _ => Err(anyhow!("{s} not valid PinentryOption")),
        }
    }
}

#[derive(Debug, Default)]
pub struct Pinentry {
    /// The window title (Assuan: "SETTITLE TITLE".)
    pub title: Option<String>,
    /// The description to display (Assuan: "SETDESC DESC".)
    pub description: Option<String>,
    /// The error message to display (Assuan: "SETERROR MESSAGE".)
    pub error: Option<String>,
    /// The prompt to display, or NULL.  (Assuan: "SETPROMPT prompt".)
    pub prompt: Option<String>,
    /// The OK button text to display (Assuan: "SETOK OK".)
    pub ok: Option<String>,
    /// The Not-OK button text to display, or NULL.  This is the text for the alternative option shown by the third button.  (Assuan: "SETNOTOK NOTOK".)
    pub notok: Option<String>,
    /// The Cancel button text to display, or NULL.  (Assuan: "SETCANCEL CANCEL".)
    pub cancel: Option<String>,
    /// The name of the terminal node to open if X not available or supported (Assuan: "OPTION ttyname TTYNAME".)
    pub ttyname: Option<String>,
}

pub trait PinentryResolver {
    fn get_pin(&mut self, pinentry: &mut Pinentry) -> Result<String, Error>;
}

impl Pinentry {
    pub fn get_resolver_fd_in_out(&self) -> Result<(File, File), Error> {
        let read = std::fs::OpenOptions::new()
            .read(true)
            .open(self.ttyname.clone().unwrap_or("/dev/stdin".to_string()))
            .map_err(|err| anyhow!("Fail open read resolver_fd_in_out: {err}"))?;
        let write = std::fs::OpenOptions::new()
            .write(true)
            .open(self.ttyname.clone().unwrap_or("/dev/stdout".to_string()))
            .map_err(|err| anyhow!("Fail open write resolver_fd_in_out: {err}"))?;
        Ok((read, write))
    }

    // pub fn tst() -> impl std::io::Read {
    //     std::io::stdin()
    //     // todo!();
    // }

    pub fn run_loop(
        mut self,
        fd_in: RawFd,
        fd_out: RawFd,
        mut resolver: impl PinentryResolver,
    ) -> Result<(), Error> {
        let mut ctx = AssuanContext::new(fd_in, fd_out)
            .register_commands(&PINENTRY_COMMANDS)
            .register_options(&PINENTRY_OPTIONS);

        // println!("commands {:?}", ctx.commands);
        // println!("options {:?}", ctx.options);
        assuan_write_line(&mut ctx, "OK pleased to meet you.").unwrap();

        loop {
            match assuan_process(&mut ctx) {
                Ok(res) => match res {
                    AssuanProcessResult::Command(cmd_str, line) => {
                        // println!("Recv cmd str {cmd_str}");
                        let Ok(cmd) = PinentryCmd::from_str(&cmd_str) else {
                            assuan_process_done(&mut ctx);
                            continue;
                        };
                        let line = unenc(&line)?;
                        match cmd {
                            PinentryCmd::SetDesc => {
                                self.description = Some(line);
                            }
                            PinentryCmd::Prompt => {
                                self.prompt = Some(line);
                            }
                            // PinentryCmd::KeyInfo => todo!(),
                            // PinentryCmd::Repeat => todo!(),
                            PinentryCmd::SetOk => {
                                self.ok = Some(line);
                            }
                            PinentryCmd::SetNotOk => {
                                self.notok = Some(line);
                            }
                            PinentryCmd::SetCancel => {
                                self.cancel = Some(line);
                            }
                            PinentryCmd::GetPin => {
                                match resolver.get_pin(&mut self) {
                                    Ok(password) => {
                                        assuan_write_line(&mut ctx, &format!("D {password}"))
                                            .expect("Fail assuan write");
                                    }
                                    Err(err) => {
                                        panic!("{err}");
                                    }
                                };
                            }
                            PinentryCmd::SetTitle => {
                                self.title = Some(line);
                            }
                            PinentryCmd::SetError => {
                                self.error = Some(line);
                            }
                            _ => {
                                // continue;
                            }
                        }
                        assuan_process_done(&mut ctx);
                    }
                    AssuanProcessResult::Option(key, value) => {
                        let Ok(option) = PinentryOption::from_str(&key) else {
                            continue;
                        };
                        match option {
                            PinentryOption::TtyName => {
                                self.ttyname = Some(value);
                            }
                        }
                    }
                    _ => {}
                },
                Err(err) => {
                    panic!("{err}")
                }
            }
        }
    }
}
