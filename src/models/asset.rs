use serde::Deserialize;

/// Defines the structure of the response coming from the BlockFrost API when getting assets.
#[derive(Deserialize, Debug)]
pub struct Asset {
    pub asset: String,
    pub src: String,
}
