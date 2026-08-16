use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
pub async fn download(client: &reqwest::Client, url: &str, dir: &Path) -> Result<PathBuf> {
    let bytes = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    if bytes.len() > 10 * 1024 * 1024 {
        anyhow::bail!("thumbnailが大きすぎます")
    };
    let ext = url::Url::parse(url)
        .ok()
        .and_then(|u| u.path_segments()?.next_back().map(str::to_owned))
        .and_then(|n| Path::new(&n).extension()?.to_str().map(str::to_owned))
        .filter(|x| ["jpg", "jpeg", "png", "webp"].contains(&x.as_str()))
        .unwrap_or_else(|| "jpg".into());
    let name = format!("{:x}.{}", Sha256::digest(url.as_bytes()), ext);
    let path = dir.join(name);
    std::fs::write(&path, bytes)
        .with_context(|| format!("thumbnailを書き込めません: {}", path.display()))?;
    Ok(path)
}
