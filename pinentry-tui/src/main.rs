use std::os::fd::AsRawFd;
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::anyhow;
use parking_lot::RwLock;
use ratatui::backend::TermionBackend;
use ratatui::layout::{Constraint, Flex, Layout, Margin, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::termion::event::Key;
use ratatui::widgets::{Block, BorderType, Paragraph, Widget};
use ratatui::{Frame, Terminal, termion};
use termion::raw::IntoRawMode;
use termion::screen::IntoAlternateScreen;
use tui_textarea::TextArea;

pub enum TermAct {
    Stop,
}

pub enum TermionEvent {
    Input(termion::event::Key),
    Tick,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub enum Focus {
    #[default]
    Input,
    Ok,
    Cancel,
}

impl Focus {
    pub fn is_input(&self) -> bool {
        self.eq(&Focus::Input)
    }

    #[must_use]
    pub fn next(&self) -> Focus {
        match self {
            Focus::Input => Self::Ok,
            Focus::Ok => Self::Cancel,
            Focus::Cancel => Self::Input,
        }
    }
}

pub struct PinentryTui {
    pub finished: Arc<RwLock<bool>>,
    pub focus: Focus,
    text_area: TextArea<'static>,
}

impl PinentryTui {
    pub fn new() -> Self {
        Self {
            finished: Arc::new(RwLock::new(false)),
            focus: Focus::default(),
            text_area: Self::text_area(),
        }
    }

    fn text_area() -> TextArea<'static> {
        let mut text_area = TextArea::default();
        text_area.set_cursor_line_style(Style::default());
        // text_area.set_placeholder_text("");
        text_area.set_mask_char('*');
        text_area.set_block(Block::bordered());
        text_area
    }

    fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
        let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
        let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
        let [area] = vertical.areas(area);
        let [area] = horizontal.areas(area);
        area
    }

    fn draw_prompt(&mut self, frame: &mut Frame, pinentry: &mut libpinentry::Pinentry) {
        let area = Self::popup_area(frame.area(), 35, 25);
        frame.render_widget(
            Block::bordered()
                .border_type(BorderType::Rounded)
                .style(Style::default().bg(Color::Rgb(100, 100, 100))),
            area,
        );

        let [desc_area, err_area, input_area, rest] = Layout::vertical([
            Constraint::Min(4),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .areas(area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        }));

        if let Some(desc) = &pinentry.description {
            frame.render_widget(
                Paragraph::new(desc.as_str())
                    .left_aligned()
                    .wrap(ratatui::widgets::Wrap { trim: true }),
                desc_area,
            );
        }

        if let Some(error) = &pinentry.error {
            frame.render_widget(
                Paragraph::new(error.as_str())
                    .left_aligned()
                    .style(Style::default().on_red())
                    .wrap(ratatui::widgets::Wrap { trim: true }),
                err_area,
            );
        }

        if self.focus.eq(&Focus::Input) {
            self.text_area.set_cursor_style(Style::default().reversed());
        } else {
            self.text_area.set_cursor_style(Style::default().hidden());
        }

        frame.render_widget(&self.text_area, input_area);

        let [ok_btn, cancel_btn] =
            Layout::horizontal([Constraint::Length(4), Constraint::Length(8)])
                .flex(Flex::SpaceBetween)
                .areas(rest.inner(Margin {
                    horizontal: 2,
                    vertical: 0,
                }));

        frame.render_widget(
            Self::btn("<OK>", self.focus.eq(&Focus::Ok)),
            ok_btn.inner(Margin {
                horizontal: 0,
                vertical: 0,
            }),
        );
        frame.render_widget(
            Self::btn("<Cancel>", self.focus.eq(&Focus::Cancel)),
            cancel_btn.inner(Margin {
                horizontal: 0,
                vertical: 0,
            }),
        );
    }

    fn btn(label: &str, in_focus: bool) -> impl Widget {
        let mut style = Style::default();
        if in_focus {
            style = style.on_dark_gray().white();
        }

        ratatui::text::Text::from(label).centered().style(style)
    }

    fn termion_events<F: std::io::Read + Send + 'static>(
        fd_in: F,
        tick_rate: Duration,
        finished: Arc<RwLock<bool>>,
    ) -> (mpsc::Receiver<TermionEvent>, Vec<JoinHandle<()>>) {
        use termion::input::TermRead;

        let (tx, rx) = mpsc::channel();
        let keys_tx = tx.clone();

        let mut joiners = vec![];
        {
            let finished = finished.clone();
            joiners.push(std::thread::spawn(move || {
                let mut events = fd_in.events().peekable();
                let mut finished_local = false;
                loop {
                    if *finished.read() {
                        break;
                    }

                    if finished_local {
                        break;
                    }

                    if events.peek().is_none() {
                        continue;
                    }
                    // fd_in.bytes().peekable().peek().is_some() {
                    // }
                    match events.next() {
                        Some(evt_res) => match evt_res {
                            Ok(evt) => match evt {
                                termion::event::Event::Key(key) => {
                                    if let Err(_) = keys_tx.send(TermionEvent::Input(key)) {
                                        finished_local = true;
                                    }
                                    match key {
                                        Key::Esc | Key::Char('\n') => finished_local = true,
                                        _ => {}
                                    }
                                }
                                _ => {}
                            },
                            Err(_) => return,
                        },
                        None => {}
                    }
                }
            }));
        }
        {
            let finished = finished.clone();
            joiners.push(std::thread::spawn(move || {
                loop {
                    if *finished.read() {
                        break;
                    }
                    if let Err(err) = tx.send(TermionEvent::Tick) {
                        // eprintln!("{err}");
                        break;
                    }
                    std::thread::sleep(tick_rate);
                }
            }));
        }
        (rx, joiners)
    }
}

impl libpinentry::PinentryResolver for PinentryTui {
    fn get_pin(
        &mut self,
        pinentry: &mut libpinentry::Pinentry,
    ) -> Result<String, libpinentry::error::Error> {
        let (fin, fout) = pinentry.get_resolver_fd_in_out()?;

        *self.finished.write() = false;
        self.text_area.delete_line_by_head();

        let backend = TermionBackend::new(fout.try_clone().unwrap());

        let fout = fout
            .into_raw_mode()
            .map_err(|err| anyhow!("Fail transform fd raw mode {err}"))?
            .into_alternate_screen()
            .map_err(|err| anyhow!("Fail into_alternate_screen {err}"))?;

        let mut terminal =
            Terminal::new(backend).map_err(|err| anyhow!("Fail init rat Terminal {err}"))?;

        let (events, evt_joiners) =
            Self::termion_events(fin, Duration::from_millis(16), self.finished.clone());

        loop {
            use termion::event::Key;

            terminal.draw(|f| self.draw_prompt(f, pinentry))?;

            match events.recv()? {
                TermionEvent::Input(key) => match key {
                    Key::Esc => {
                        panic!("Operation was cancelled")
                    }
                    Key::Char('\n') => {
                        if self.focus.eq(&Focus::Cancel) {
                            panic!("Operation was cancelled")
                        } else {
                            break;
                        }
                    }
                    Key::Char('\t') => {
                        self.focus = self.focus.next();
                    }
                    Key::Ctrl('w') => {
                        if self.focus.is_input() {
                            self.text_area.delete_word();
                        }
                    }
                    Key::Ctrl('u') => {
                        if self.focus.is_input() {
                            self.text_area.delete_line_by_head();
                        }
                    }
                    Key::Backspace => {
                        if self.focus.is_input() {
                            self.text_area.delete_char();
                        }
                    }
                    Key::Char(c) => {
                        if self.focus.is_input() {
                            // TODO
                            self.text_area.insert_char(c);
                            // self.text_area.input(key);
                        }
                    }
                    _ => {}
                },
                TermionEvent::Tick => {}
            }
        }

        *self.finished.write() = true;

        let secret_input = self
            .text_area
            .lines()
            .first()
            .unwrap_or(&String::new())
            .to_string();

        for joiner in evt_joiners {
            joiner.join().ok();
        }

        // terminal
        //     .clear()
        //     .map_err(|err| anyhow!("Fail to clear terminal. {err}"))?;

        Ok(secret_input)
    }
}

fn main() {
    let resolver = PinentryTui::new();

    let fd_in = std::io::stdin().as_raw_fd();
    let fd_out = std::io::stdout().as_raw_fd();

    libpinentry::Pinentry::default()
        .run_loop(fd_in, fd_out, resolver)
        .ok();
}
