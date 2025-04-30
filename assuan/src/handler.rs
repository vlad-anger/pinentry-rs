use std::{
    io::{BufRead, BufReader, Write},
    str::FromStr,
};

use anyhow::anyhow;

use crate::{error::Error, logw, AssuanContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssuanStdCmd {
    Nop,
    Cancel,
    Option,
    Bye,
    Auth,
    Reset,
    End,
    Help,
    Input,
    Output,
    Sendfd,
}

impl FromStr for AssuanStdCmd {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "NOP" => Ok(Self::Nop),
            "CANCEL" => Ok(Self::Cancel),
            "OPTION" => Ok(Self::Option),
            "BYE" => Ok(Self::Bye),
            "AUTH" => Ok(Self::Auth),
            "RESET" => Ok(Self::Reset),
            "END" => Ok(Self::End),
            "HELP" => Ok(Self::Help),
            "INPUT" => Ok(Self::Input),
            "OUTPUT" => Ok(Self::Output),
            "SENDFD" => Ok(Self::Sendfd),
            _ => Err(anyhow!("{s} is invalid AssuanStdCmd")),
        }
    }
}

pub enum AssuanProcessResult {
    Command(String, String),
    Option(String, String),
    Unknown,
}

pub fn assuan_accept(ctx: &mut AssuanContext) {
    // todo!();
}

pub fn assuan_process(ctx: &mut AssuanContext) -> Result<AssuanProcessResult, Error> {
    process_request(ctx)
}

fn process_request(ctx: &mut AssuanContext) -> Result<AssuanProcessResult, Error> {
    let msg = assuan_read_line(ctx)?;
    let mut parts = msg.splitn(2, ' ');
    logw(&format!("Receive {msg}"));
    if let Some(p1) = parts.next() {
        if let Ok(std_cmd) = AssuanStdCmd::from_str(p1) {
            match std_cmd {
                AssuanStdCmd::Nop => {
                    assuan_process_done(ctx);
                }
                AssuanStdCmd::Cancel => {
                    assuan_process_done_err(ctx, anyhow!("End. Not implemented"));
                }
                AssuanStdCmd::Option => {
                    assuan_process_done(ctx);
                    if let Some(pparts) = parts.next().map(|v| v.splitn(2, '=').collect::<Vec<_>>())
                    {
                        if pparts.len() >= 2 {
                            return Ok(AssuanProcessResult::Option(
                                pparts.first().unwrap().to_string(),
                                pparts.get(1).unwrap().to_string(),
                            ));
                        }
                    }
                }
                AssuanStdCmd::Bye => {
                    assuan_process_done_exit(ctx);
                }
                // AssuanStdCmd::Auth => todo!(),
                // AssuanStdCmd::Reset => {
                //     assuan_process_done_err(ctx, anyhow!("Reset"));
                // }
                AssuanStdCmd::End => {
                    assuan_process_done_err(ctx, anyhow!("End. Not implemented"));
                }
                // AssuanStdCmd::Help => todo!(),
                // AssuanStdCmd::Input => todo!(),
                // AssuanStdCmd::Output => todo!(),
                _ => {
                    assuan_process_done(ctx);
                    return Ok(AssuanProcessResult::Unknown);
                }
            }
        }

        if let Some(cmd) = &ctx.commands.iter().find(|cmd| cmd.eq(&p1)).cloned() {
            return Ok(AssuanProcessResult::Command(
                cmd.to_string(),
                parts.next().unwrap_or_default().to_string(),
            ));
        }
    }

    assuan_process_done(ctx);
    Ok(AssuanProcessResult::Unknown)
}

pub fn assuan_process_done(ctx: &mut AssuanContext) {
    assuan_write_line(ctx, "OK").expect("Fail write line to outbound fd");
}

pub fn assuan_process_done_exit(ctx: &mut AssuanContext) {
    assuan_process_done(ctx);
    std::process::exit(0);
}

pub fn assuan_process_done_err(ctx: &mut AssuanContext, err: Error) {
    panic!("{err}")
}

pub fn assuan_read_line(ctx: &AssuanContext) -> Result<String, Error> {
    let f_in = &ctx.inbound.fd;
    let mut buf_reader = BufReader::new(f_in);
    let mut line = String::new();
    buf_reader.read_line(&mut line)?;
    Ok(line.trim().to_string())
}

pub fn assuan_write_line(ctx: &mut AssuanContext, line: &str) -> Result<(), Error> {
    let f_out = &mut ctx.outbound.fd;
    f_out.write_all(format!("{line}\n").as_bytes())?;
    Ok(())
}
