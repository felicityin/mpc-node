use anyhow::{Context, Result};

#[tokio::main]
pub async fn run(
    _server_url: &str,
    _room: &str,
    local_share: &[u8],
    parties: Vec<u16>,
    _data: &[u8],
    _index: u16,
) -> Result<()> {
    let _local_share = serde_json::from_slice(local_share).context("parse local share")?;
    let _number_of_parties = parties.len();

    // todo

    Ok(())
}
