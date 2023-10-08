use anyhow::{Context, Result};
use reqwest::Url;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

/// DownloadService: it has one member: output_dir.
pub struct DownloadService {
    output_dir: PathBuf,
}

impl DownloadService {
    /// Constructor for DownloadService. Takes a path where files will be saved.
    pub fn new(output_dir: impl Into<PathBuf>) -> Self {
        Self { output_dir: output_dir.into() }
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
            let mut source = reqwest::get(url).await.context("Failed downloading file")?;

            // Initialize an async file instance pointing at the temp file
            let mut dest =
                File::create(&temp_file_path).await.context("Failed creating destination file")?;

            // Stream download
            // While there are data chunks available in the source...
            while let Ok(chunk_result) = source.chunk().await {
                match chunk_result {
                    Some(chunk) => {
                        // ...write those chunks to the destination file.
                        dest.write_all(&chunk).await.context("Failed to write to the file")?;
                    }
                    None => (),
                }
            }

            // Persisting temp file (rename)
            // Once the temp file is fully written, rename it to the desired filename.
            tokio::fs::rename(temp_file_path, &output_path)
                .await
                .context("Failed to persist temp file")?;
        }

        Ok(())
    }
}
