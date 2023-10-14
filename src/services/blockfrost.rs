//! This module defines a service for interacting with BlockFrost API. It provides methods to fetch assets and download images from given policy id.

use crate::{models::asset::Asset, utils::util::ipfs_to_http};

use super::download::DownloadService;
use anyhow::{Context, Result};
use blockfrost::{load, AssetDetails, BlockFrostApi, BlockFrostSettings};
use futures::future;
use reqwest::Url;
use serde_json::Value;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::mpsc;
use tokio::sync::Semaphore;

/// BlockFrostService: struct to encapsulate methods related
/// to interacting with BlockFrost API, an API to fetch Cardano data.
pub struct BlockFrostService {
    client: BlockFrostApi,
}

const NUM_CONCURRENT_FETCHES: usize = 1;

impl BlockFrostService {
    /// Constructs a new BlockFrostService.
    ///
    /// Initializes blockfrost api client with provided configurations.
    pub fn new() -> Result<Self> {
        let configurations =
            load::configurations_from_env().context("Failed to load configurations")?;
        let project_id = configurations["project_id"].as_str().unwrap();
        let mut settings = BlockFrostSettings::new();
        // Limit quantity of items per page listed
        settings.query_parameters.set_count(20);
        let client = BlockFrostApi::new(project_id, settings);
        Ok(BlockFrostService { client })
    }

    /// Fetches all assets pertaining to a policy_id from BlockFrost.
    ///
    /// Returns a vector of fetched asset ids.
    pub async fn fetch_assets(&self, policy_id: &str) -> Result<Vec<String>> {
        let result =
            self.client.assets_policy_by_id(policy_id).await.context("Failed to fetch assets")?;
        let assets: Vec<String> = result.iter().map(|item| item.asset.clone()).collect();
        return Ok(assets);
    }

    /// Fetches metadata of all assets pertaining to a policy_id.
    ///
    /// Downloads associated image pertaining to the asset metadata.
    ///
    /// Returns a vector of Assets i.e., metadata fetched and a url to the downloaded image.
    pub async fn fetch_assets_metadata(
        &self,
        policy_id: &str,
        output_dir: &PathBuf,
    ) -> Result<Vec<Asset>> {
        // fetch all assets related to given policy
        let mut assets = self.fetch_assets(policy_id).await.context("Failed to fetch assets")?;
        assets.reverse();

        let (tx, mut rx) =
            mpsc::channel::<Result<AssetDetails, blockfrost::Error>>(NUM_CONCURRENT_FETCHES);
        let mut asset_metadata = vec![];
        let mut download_tasks = Vec::new();
        let initial_assets = assets.split_off(assets.len().saturating_sub(NUM_CONCURRENT_FETCHES));
        let mut remaining_tasks = initial_assets.len();

        // create a semaphore to limit the number of concurrent downloads (limit the cpu usage)
        let semaphore = Arc::new(Semaphore::new(3));

        let download_service = Arc::new(DownloadService::new(output_dir.clone()));

        let client = Arc::new(self.client.clone());
        for asset in initial_assets {
            // create a new task for each asset in order fetch the asset details
            // concurrently and thereby improving the throughput of the system
            let tx = tx.clone();
            let client = Arc::clone(&client);
            tokio::spawn(async move {
                let asset_metadata = client.assets_by_id(&asset).await;
                match tx.send(asset_metadata).await {
                    Err(e) => eprintln!("Failed to send asset metadata: {:?}", e),
                    _ => (),
                };
            });
        }

        while let Some(result) = rx.recv().await {
            remaining_tasks = remaining_tasks.saturating_sub(1);
            let mut is_valid = false;
            if let Ok(metadata) = result {
                if let Some(onchain_metadata) = metadata.onchain_metadata {
                    // checking if the downloaded data object is the image
                    if let Some(Value::Array(files)) = onchain_metadata.get("files") {
                        // check if the download source is available for the field
                        if let Some(Value::String(src)) = files[0].get("src") {
                            // check if the source is available
                            let url = ipfs_to_http(&src);
                            if let Ok(url) = url {
                                asset_metadata.push(Asset {
                                    asset: metadata.asset.clone(),
                                    src: src.clone(),
                                });
                                is_valid = true;

                                let mut extension = "png";
                                if let Some(Value::String(media_type)) = files[0].get("mediaType") {
                                    match media_type.as_str() {
                                        "image/png" => {
                                            extension = "png";
                                        }
                                        _ => (),
                                    }
                                }
                                let asset = metadata.asset.clone();
                                let filename = format!("{}.{}", asset, extension);

                                // create a new task to download the image associated with the asset
                                let permit = semaphore
                                    .clone()
                                    .acquire_owned()
                                    .await
                                    .expect("Failed to acquire semaphore"); // Acquire a permit from the semaphore

                                let download_service = Arc::clone(&download_service);

                                println!("Downloading asset: {:?}", url);
                                let url = Url::parse(&url)?;
                                let download_task = tokio::spawn(async move {
                                    let _permit = permit;
                                    match download_service.download_and_save(url, filename).await {
                                        Err(e) => eprintln!("Failed to download asset: {:?}", e),
                                        _ => (),
                                    }
                                });

                                download_tasks.push(download_task);
                            }
                        }
                    }
                }
            }

            if !is_valid {
                if let Some(asset) = assets.pop() {
                    let tx = tx.clone();
                    remaining_tasks += 1;
                    let client = self.client.clone();
                    tokio::spawn(async move {
                        let asset_metadata = client.assets_by_id(&asset).await;
                        tx.send(asset_metadata).await.unwrap();
                    });
                }
            }

            if remaining_tasks == 0 {
                break;
            }
        }

        future::join_all(download_tasks).await;

        Ok(asset_metadata)
    }
}
