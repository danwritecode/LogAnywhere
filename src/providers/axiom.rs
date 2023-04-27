use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::header;
use crate::{LogProvider, LogAnywhereRecord};
use async_trait::async_trait;

pub struct AxiomProvider {
    auth_token: String,
    dataset: String
}

impl AxiomProvider {
    pub fn new(auth_token: String, dataset: String) -> AxiomProvider {
        AxiomProvider {
            auth_token,
            dataset
        }
    }
}

#[async_trait]
impl LogProvider for AxiomProvider {
    async fn send_log(&self, messages: Vec<LogAnywhereRecord>) {
        let mut headers = header::HeaderMap::new();
        headers.insert(AUTHORIZATION, format!("Bearer {}", &self.auth_token).parse().unwrap());
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

        let client = reqwest::Client::new();
        let url = format!("https://api.axiom.co/v1/datasets/{}/ingest", self.dataset);
        let res = client.post(url)
            .headers(headers)
            .json(&messages)
            .send()
            .await;

        match res {
            Ok(res) => println!("res: {:?}", res.text().await.unwrap()),
            Err(e) => {
                println!("error status: {:?}, error: {:?}", e.status(), e)
            }
        }
    }
}
