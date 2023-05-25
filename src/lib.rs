pub mod providers;
pub use crate::providers::prelude::*;

use std::sync::{ Arc, Mutex };
use std::time::Duration;
use std::{ mem, panic };

use log::{Log, SetLoggerError};
use anyhow::Result;
use serde::Serialize;
use async_trait::async_trait;

use tokio::task;
use tokio::time::sleep;



#[async_trait]
pub trait LogProvider: Send + Sync {
    async fn send_log(&self, messages: Vec<LogAnywhereRecord>);
}


#[derive(Clone)]
pub struct Logger {
    providers: Vec<Arc<dyn LogProvider>>,
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
        *is_panicking.lock().unwrap() = true;
        
        eprintln!("{}", p);
        eprintln!("waiting for log_anywhere to cleanup, 1 second please");

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

impl Logger {
    /// Initializes the global logger with a LogAnywhereLogger instance.
    ///
    /// ```no_run
    /// let mut providers:Vec<Arc<dyn LogProvider>> = vec![];
    /// providers.push(AxiomProvider::new(token, dataset));
    ///
    /// let logger = Logger::new(providers, 5, LevelFilter::Info);
    /// logger.init()?;
    /// ```
    ///
    /// provider: any provider implementing the LogProvider trait. You can create your own. 
    /// buffer_timing: LogAnywhereLogger uses a buffer to batch send log messages.
    /// Given this, buffer_timing represents the time between batched shipments of your logs.
    pub fn new(
        providers: Vec<Arc<dyn LogProvider>>,
        buffer_timing: u64,
        level: log::LevelFilter
    ) -> Self {
        Logger {
            providers,
            log_buffer_records: Arc::new(Mutex::new(Vec::new())),
            buffer_timing: Arc::new(buffer_timing),
            buffer_emptied_on_panic: Arc::new(Mutex::new(false)),
            is_panicking: Arc::new(Mutex::new(false)),
            level: Arc::new(level),
        }
    }

    pub fn init(self: Self) -> Result<(), SetLoggerError> {
        let level_ptr = Arc::clone(&self.level);

        // set panic hook
        set_panic_hook(
            self.log_buffer_records.clone(), 
            self.buffer_emptied_on_panic.clone(), 
            self.is_panicking.clone()
        );

        for provider in &self.providers {
            task::spawn(
                buffer_loop(
                    self.log_buffer_records.clone(), 
                    provider.clone(),
                    self.buffer_timing.clone(), 
                    self.buffer_emptied_on_panic.clone(), 
                    self.is_panicking.clone()
                )
            );
        }

        let boxed_self = Box::new(self);
        log::set_boxed_logger(boxed_self)?;
        log::set_max_level(*level_ptr);
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct LogAnywhereRecord {
    pub level: String,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>
}

unsafe impl Sync for Logger {}
unsafe impl Send for Logger {}

impl Log for Logger {
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

        println!("{} | message: {} | line: {:?}", anywhere_log.level, anywhere_log.message, anywhere_log.line);
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

