pub mod providers;
pub use crate::providers::prelude::*;

use std::sync::{ Arc, Mutex };
use std::time::Duration;
use std::{ mem, panic, thread };

use log::{Log, SetLoggerError};
use anyhow::Result;
use serde::Serialize;
use async_trait::async_trait;
use crossbeam::channel;

use tokio::runtime::Runtime;
use tokio::task;
use tokio::time::sleep;



#[async_trait]
pub trait LogProvider: Send + Sync {
    async fn send_log(&self, messages: Vec<LogAnywhereRecord>);
}


#[derive(Clone)]
pub struct LogAnywhereLogger {
    provider: Arc<dyn LogProvider>,
    log_buffer_records: Arc<Mutex<Vec<LogAnywhereRecord>>>,
    buffer_timing: Arc<u64>,
    buffer_emptied_on_panic: Arc<Mutex<bool>>,
    is_panicking: Arc<Mutex<bool>>,
    level: Arc<log::LevelFilter> // TO DO: Make this into string slice
}

async fn buffer_loop(
    log_buffer_records: Arc<Mutex<Vec<LogAnywhereRecord>>>, 
    provider: Arc<dyn LogProvider>, 
    buffer_timing: Arc<u64>,
    buffer_emptied_on_panic: Arc<Mutex<bool>>,
    is_panicking: Arc<Mutex<bool>>
) {
    loop {
        let messages = {
            let mut records_guard = log_buffer_records.lock().unwrap();
            mem::take(&mut *records_guard)
        };

        if messages.len() > 0 {
            provider.send_log(messages).await;

            if *is_panicking.lock().unwrap() {
                println!("panic state detected");
                if log_buffer_records.lock().unwrap().len() == 0 {
                    println!("buffer empty in panic, exiting");
                    *buffer_emptied_on_panic.lock().unwrap() = true;
                } else {
                    println!("buffer not empty, waiting for next loop cycle to empty buffer");
                }
            }
        }
        sleep(Duration::from_secs(*buffer_timing)).await
    }
}

fn set_panic_hook (
    log_buffer_records: Arc<Mutex<Vec<LogAnywhereRecord>>>,
    buffer_emptied_on_panic: Arc<Mutex<bool>>,
    is_panicking: Arc<Mutex<bool>>
) {

    panic::set_hook(Box::new(move |p| {
        eprintln!("{}", p);
        eprintln!("waiting for log_anywhere to cleanup, 1 second please");
        *is_panicking.lock().unwrap() = true;

        let file = p.location().map(|l| l.file().to_string());
        let line = p.location().map(|l| l.line());

        let anywhere_log = LogAnywhereRecord {
            level: "PANIC".to_string(),
            message: p.to_string(),
            file,
            line
        };

        log_buffer_records.lock().unwrap().push(anywhere_log);

        // loop infinitely until buffer is emptied
        while !*buffer_emptied_on_panic.lock().unwrap() {}
    }));
}

impl LogAnywhereLogger {
    /// Initializes the global logger with a LogAnywhereLogger instance.
    ///
    /// ```no_run
    /// let provider = Arc::new(AxiomProvider::new(token, dataset));
    ///
    /// let logger = LogAnywhereLogger::new(provider, 5, LevelFilter::Info);
    /// let boxed_logger = Box::new(logger);
    /// boxed_logger.init().unwrap();
    /// ```
    ///
    /// provider: any provider implementing the LogProvider trait. You can create your own. 
    /// buffer_timing: LogAnywhereLogger uses a buffer to batch send log messages.
    /// Given this, buffer_timing represents the time between batched shipments of your logs.
    pub fn new(
        provider: Arc<dyn LogProvider>, 
        buffer_timing: u64,
        level: log::LevelFilter
    ) -> Self {
        LogAnywhereLogger {
            provider,
            log_buffer_records: Arc::new(Mutex::new(Vec::new())),
            buffer_timing: Arc::new(buffer_timing),
            buffer_emptied_on_panic: Arc::new(Mutex::new(false)),
            is_panicking: Arc::new(Mutex::new(false)),
            level: Arc::new(level),
        }
    }

    pub fn init(self: Box<Self>) -> Result<(), SetLoggerError> {
        let level_ptr = Arc::clone(&self.level);

        // set panic hook
        set_panic_hook(
            self.log_buffer_records.clone(), 
            self.buffer_emptied_on_panic.clone(), 
            self.is_panicking.clone()
        );

        // start buffer_loop
        task::spawn(
            buffer_loop(
                self.log_buffer_records.clone(), 
                self.provider.clone(), 
                self.buffer_timing.clone(), 
                self.buffer_emptied_on_panic.clone(), 
                self.is_panicking.clone()
            )
        );

        log::set_boxed_logger(self)?;
        log::set_max_level(*level_ptr);
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct LogAnywhereRecord {
    level: String,
    message: String,
    file: Option<String>,
    line: Option<u32>
}

unsafe impl Sync for LogAnywhereLogger {}
unsafe impl Send for LogAnywhereLogger {}

impl Log for LogAnywhereLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let file = match record.file() {
            Some(f) => Some(f.to_string()),
            None => None
        };

        let anywhere_log = LogAnywhereRecord {
            level: record.level().to_string(),
            message: record.args().to_string(),
            file,
            line: record.line()
        };
        self.log_buffer_records.lock().unwrap().push(anywhere_log);
    }

    fn flush(&self) {
        
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // let result = init(2, 2);
        assert_eq!(4, 4);
    }
}

