pub const IPFS_GATEWAY: &str = "https://ipfs.io/ipfs/";

pub fn ipfs_to_http(ipfs_hash: &str) -> Result<String, &'static str> {
    if ipfs_hash.starts_with("ipfs://") {
        let hash = &ipfs_hash[7..];
        Ok(format!("{}{}", IPFS_GATEWAY, hash))
    } else {
        Err("Invalid IPFS hash, must start with 'ipfs://' prefix")
    }
}
