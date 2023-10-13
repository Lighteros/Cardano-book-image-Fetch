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

pub const URL: &str = "https://api.book.io/api/v0/collections";

impl BookioService {
    /// Constructs a new `BookioService`.
    pub fn new() -> Result<Self> {
        let client = Client::new();
        Ok(BookioService { client })
    }

    /// Fetches collections from the Book.io API.
    pub async fn fetch_collections(&self, url: &str) -> Result<Vec<Collection>> {
        let res = self.client.get(url).send().await.context("Failed to fetch collections")?;
        let response = res.json::<CollectionResponse>().await.context("Failed to parse")?;
        Ok(response.data)
    }

    /// Verifies the given policy ID by checking it's presence in the collections data fetched from the Book.io API.
    pub async fn verify_policy_id(&self, policy_id: &str, url: &str) -> Result<bool> {
        match self.fetch_collections(url).await {
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

#[cfg(test)]
mod tests {
    use super::*;
    use mockito;

    #[tokio::test]
    async fn test_fetch_collections() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body(
                r#"
                {
                  "type": "collections",
                  "data": [
                    {
                      "collection_id": "test_id_1",
                      "description": "description 1",
                      "blockchain": "blockchain 1",
                      "network": "network 1"
                    },
                    {
                      "collection_id": "test_id_2",
                      "description": "description 2",
                      "blockchain": "blockchain 2",
                      "network": "network 2"
                    }
                  ]
                }
            "#,
            )
            .create();
        let url = server.url();
        let service = BookioService::new().unwrap();
        let collections = service.fetch_collections(&url).await.unwrap();
        assert_eq!(collections.len(), 2);

        let first = &collections[0];
        assert_eq!(first.collection_id, "test_id_1");
        assert_eq!(first.description, "description 1");

        mock.assert();
    }

    #[tokio::test]
    async fn test_verify_policy_id() {
        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body(
                r#"
                {
                  "type": "collections",
                  "data": [
                    {
                      "collection_id": "test_id_1",
                      "description": "description 1",
                      "blockchain": "blockchain 1",
                      "network": "network 1"
                    },
                    {
                      "collection_id": "test_id_2",
                      "description": "description 2",
                      "blockchain": "blockchain 2",
                      "network": "network 2"
                    }
                  ]
                }
            "#,
            )
            .create();
        let url = server.url();
        let service = BookioService::new().unwrap();
        let is_valid = service.verify_policy_id("test_id_1", &url).await.unwrap();
        assert_eq!(is_valid, true);
        mock.assert();
    }
}
