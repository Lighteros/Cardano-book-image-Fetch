//! This module defines a service for interacting with Book.io API. It provides methods to fetch collections and verify policy IDs.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;

/// BookioService: Defines the structure of a collection from the Book.io API.
#[derive(Deserialize, Debug)]
pub struct Collection {
    pub collection_id: String,
    pub description: String,
    pub blockchain: String,
    pub network: String,
}

/// Defines the structure of the response coming from the Book.io API when getting collections.

#[derive(Deserialize, Debug)]
pub struct CollectionResponse {
    pub r#type: String,
    pub data: Vec<Collection>,
}

/// Service to interact with the Book.io API.
pub struct BookioService {
    client: Client,
}

const URL: &str = "https://api.book.io/api/v0/collections";

impl BookioService {
    /// Constructs a new `BookioService`.
    pub async fn new() -> Result<Self> {
        let client = Client::new();
        Ok(BookioService { client })
    }

    /// Fetches collections from the Book.io API.
    pub async fn fetch_collections(&self) -> Result<Vec<Collection>> {
        let res = self.client.get(URL).send().await.context("Failed to fetch collections")?;
        let response = res.json::<CollectionResponse>().await.context("Failed to parse")?;
        Ok(response.data)
    }

    /// Verifies the given policy ID by checking it's presence in the collections data fetched from the Book.io API.
    pub async fn verify_policy_id(&self, policy_id: &str) -> Result<bool> {
        match self.fetch_collections().await {
            Ok(collections) => {
                if collections.iter().find(|col| col.collection_id == policy_id).is_some() {
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Err(e) => Err(e),
        }
    }
}
