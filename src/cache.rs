use std::path::Path;

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use tokio::{
    fs::{self, File},
    io::AsyncReadExt,
};

use crate::{ExpiringValue, MSATokenResponce, XSTSToken};

pub fn create_hash(user_name: &str) -> String {
    let mut sha256 = Sha256::new();
    sha256.update(user_name);
    sha256.finalize()[0..10]
        .iter()
        .map(|n| format!("{n:02x}"))
        .collect::<Vec<String>>()
        .join("")
}
pub async fn get_xsts_cache(
    cache_path: &Path,
    user_hash: &str,
) -> Result<ExpiringValue<XSTSToken>> {
    let path = cache_path.join(format!("{}_xbl-cache.json", user_hash));
    let mut buffer = Vec::new();
    File::open(path).await?.read_to_end(&mut buffer).await?;
    serde_json::from_slice(&buffer).map_err(|e| anyhow!("{e:?}"))
}
pub async fn update_xsts_cache(
    cache_path: &Path,
    user_hash: &str,
    xsts: &ExpiringValue<XSTSToken>,
) -> Result<()> {
    let path = cache_path.join(format!("{}_xbl-cache.json", user_hash));
    let json = serde_json::to_string(&xsts).map_err(|e| anyhow!("{e:?}"))?;
    fs::write(path, json).await?;
    Ok(())
}
pub async fn get_msa_cache(
    cache_path: &Path,
    user_hash: &str,
) -> Result<ExpiringValue<MSATokenResponce>> {
    let path = cache_path.join(format!("{}_msa-cache.json", user_hash));
    let mut buffer = Vec::new();
    File::open(path).await?.read_to_end(&mut buffer).await?;
    serde_json::from_slice(&buffer).map_err(|e| anyhow!("{e:?}"))
}
pub async fn update_msa_cache(
    cache_path: &Path,
    user_hash: &str,
    msa: &ExpiringValue<MSATokenResponce>,
) -> Result<()> {
    let path = cache_path.join(format!("{}_msa-cache.json", user_hash));
    let json = serde_json::to_string(&msa).map_err(|e| anyhow!("{e:?}"))?;
    fs::write(path, json).await?;
    Ok(())
}
