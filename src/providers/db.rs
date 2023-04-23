use async_trait::async_trait;
use crate::{LogProvider, LogAnywhereRecord};

pub struct DbProvider {
    db_conn: String
}

impl DbProvider {
    pub fn new() -> DbProvider {
        let db_conn = "".to_string();
        DbProvider {
            db_conn
        }
    }
}

#[async_trait]
impl LogProvider for DbProvider {
    async fn send_log(&self, messages: Vec<LogAnywhereRecord>) {
        println!("DB logged for DB: {:?}", messages);
    }
}
