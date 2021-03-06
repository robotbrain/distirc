#[macro_use] extern crate log;
#[macro_use] extern crate rotor;
extern crate env_logger;
extern crate rotor_stream;
extern crate rustbox;
extern crate time;
extern crate rustc_serialize;
extern crate toml;
extern crate xdg;

extern crate common;

use std::sync::Mutex;
use std::net::ToSocketAddrs;
use log::{Log, LogLevelFilter, LogRecord, LogMetadata, MaxLogLevelFilter};

use common::line::{BufferLine, LineData, MsgKind};
use common::messages::Password;

pub mod ui;
pub mod model;
pub mod conn;
pub mod config;

use self::ui::TermUi;
use self::conn::ConnThread;
use self::model::{Buffer, BufSender, BufKey};
use self::config::read_config;

fn main() {
    let cfg = read_config();

    // env_logger::init().expect("Failed to initialize logger");
    let (buf, bs) = Buffer::new(BufKey::Status);
    ClientLogger::init(bs, LogLevelFilter::Trace);
    info!("Hello! Welcome to distirc's terminal client.");

    let addr = (&cfg.core.host[..], cfg.core.port).to_socket_addrs()
        .unwrap().into_iter().next().unwrap();

    let pass = Password(cfg.core.pass.clone());
    let conn = ConnThread::spawn(addr, cfg.core.user.clone(), pass);

    let mut ui = TermUi::new(buf, conn).expect("Failed to initialize UI");
    ui.main();
}


/// A logger that writes to a buffer handle.
struct ClientLogger {
    bs: Mutex<BufSender>,
    filter: MaxLogLevelFilter,
}

impl ClientLogger {
    pub fn init(bs: BufSender, level: LogLevelFilter) {
        log::set_logger(move |filter| { // TODO: Use filter
            filter.set(level);
            let l = ClientLogger {
                bs: Mutex::new(bs),
                filter: filter,
            };
            Box::new(l) as Box<Log>
        }).expect("Failed to initialize logging system");
    }
}

impl Log for ClientLogger {
    fn enabled(&self, meta: &LogMetadata) -> bool {
        meta.level() <= self.filter.get() &&
            (meta.target().starts_with("distirc")
             || meta.target().starts_with("client")
             || meta.target().starts_with("common")
            )
    }

    fn log(&self, log: &LogRecord) {
        if self.enabled(&log.metadata()) {
            let msg = format!(
                "{0: >5} {1} {2}",
                log.level(),
                log.location().module_path(),
                log.args());
            let data = LineData::Message {
                from: "status".to_owned(),
                msg: msg,
                kind: MsgKind::Status,
            };

            let line = BufferLine::new(time::now(), data);

            let mut bs = self.bs.lock().expect("Failed to lock log destination mutex");
            bs.send_front(line);
        }
    }
}
