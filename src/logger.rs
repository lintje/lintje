use log::{Level, Metadata, Record};

pub struct Logger {
    level: log::Level,
}

impl Logger {
    pub fn new() -> Self {
        Self { level: Level::Warn }
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        println!("[{}] {}", record.level(), record.args());
    }

    fn flush(&self) {}
}
