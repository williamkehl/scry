use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

fn config_dir() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("Could not find config directory")?
        .join("scry");
    fs::create_dir_all(&dir)
        .context("Failed to create config directory")?;
    Ok(dir)
}

fn key_file() -> Result<PathBuf> {
    Ok(config_dir()?.join("api_key"))
}

pub fn get_api_key() -> Result<String> {
    let key_path = key_file()?;
    fs::read_to_string(&key_path)
        .context("API key not set. Run 'scry key YOUR_API_KEY' to set it.")
}

pub fn has_api_key() -> bool {
    key_file()
        .and_then(|path| {
            fs::read_to_string(path)
                .map(|s| !s.trim().is_empty())
                .map_err(|e| anyhow::anyhow!("{}", e))
        })
        .unwrap_or(false)
}

pub fn set_api_key(key: &str) -> Result<()> {
    let key_path = key_file()?;
    fs::write(&key_path, key.trim())
        .context("Failed to write API key to config file")?;
    Ok(())
}

pub fn delete_api_key() -> Result<()> {
    let key_path = key_file()?;
    if key_path.exists() {
        fs::remove_file(&key_path)
            .context("Failed to delete API key file")?;
    }
    Ok(())
}

