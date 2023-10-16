use anyhow::{Context, Result};
use bytes::Bytes;
use reqwest::{Client, Url};
use std::path::PathBuf;
use tempfile::NamedTempFile;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio_stream::StreamExt;

/// DownloadService: it has one member: output_dir.
pub struct DownloadService {
    output_dir: PathBuf,
    client: Client,
}

impl DownloadService {
    /// Constructor for DownloadService. Takes a path where files will be saved.
    pub fn new(output_dir: impl Into<PathBuf>) -> Self {
        Self { output_dir: output_dir.into(), client: Client::new() }
    }

    /// The method responsible for downloading and saving a file.
    ///
    /// 'url' is the web location of the file to fetch, 'filename' is the name
    /// for the file once it's saved locally.
    pub async fn download_and_save(&self, url: Url, filename: String) -> Result<()> {
        // Clone the path to the output directory and push filename onto it
        let mut output_path = self.output_dir.clone();
        output_path.push(&filename);

        // If the output directory doesn't exist yet, create it
        if !self.output_dir.exists() {
            tokio::fs::create_dir_all(&self.output_dir)
                .await
                .context("Failed to create the output directory")?;
        }

        // Only proceed with fetch and write operations if the file doesn't exist in
        // the output directory yet.
        if !output_path.exists() {
            // Create temporary file in the output directory
            let temp_file =
                NamedTempFile::new_in(&self.output_dir).context("Failed to create temp file")?;

            // Record temporary file's path for later operations
            let temp_file_path = temp_file.path().to_path_buf(); // keep tempfile path for later

            // Fetch the file content into the 'source' variable
            let response = self.client.get(url).send().await.context("Failed downloading file")?;

            let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<Bytes, reqwest::Error>>(1024);

            // Initialize an async file instance pointing at the temp file
            let mut dest =
                File::create(&temp_file_path).await.context("Failed creating destination file")?;

            // Spawn a task to read from the network
            let network_reader = tokio::spawn(async move {
                let mut stream = response.bytes_stream();
                while let Some(chunk) = stream.next().await {
                    tx.send(chunk).await.context("Failed to send chunk to the channel")?;
                }
                Result::<(), anyhow::Error>::Ok(())
            });
            // Stream download
            // While there are data chunks available in the source...
            while let Some(chunk) = rx.recv().await {
                let chunk = chunk.context("Failed reading chunk from the stream")?;
                dest.write_all(&chunk).await.context("Failed to write chunk to output")?;
            }

            network_reader.await.context("Network read task failed")??;

            // let bytes = response.bytes().await.context("failed converting respone to bytes")?;
            // dest.write_all(&bytes).await.context("Failed to write to the file")?;

            // Persisting temp file (rename)
            // Once the temp file is fully written, rename it to the desired filename.
            tokio::fs::rename(temp_file_path, &output_path)
                .await
                .context("Failed to persist temp file")?;
        }

        Ok(())
    }
}
