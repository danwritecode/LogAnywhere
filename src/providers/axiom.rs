use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use reqwest::header;
use serde::{Deserialize, Serialize};
use anyhow::Result;

use crate::{LogProvider, LogAnywhereRecord};
use async_trait::async_trait;

use std::sync::{ Arc, Mutex };

pub struct AxiomProvider<'a> {
    auth_token: &'a str,
    dataset: &'a str
}

impl<'a> AxiomProvider<'a> {
    pub fn new(auth_token: &'a str, dataset: &'a str) -> AxiomProvider<'a> {
        AxiomProvider {
            auth_token,
            dataset
        }
    }
}

#[async_trait]
impl<'a> LogProvider for AxiomProvider<'a> {
    async fn send_log(&self, messages: Vec<LogAnywhereRecord>) {
        println!("Logged for Axiom: {:?}", messages);

        let mut headers = header::HeaderMap::new();
        headers.insert(AUTHORIZATION, format!("Bearer {}", &self.auth_token).parse().unwrap());
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

        let client = reqwest::Client::new();
        let url = "https://api.axiom.co/v1/datasets/worker_logs/ingest";
        let res = client.post(url)
            .headers(headers)
            .json(&messages)
            .send()
            .await
            .unwrap();

        println!("res: {:?}", res.text().await.unwrap());
    }
}
