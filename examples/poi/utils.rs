// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::{fs, fs::File, path::PathBuf};

use anyhow::{Context, Result};
use iota_config::{IOTA_GENESIS_FILENAME, iota_config_dir};

const MAINNET_GENESIS_URL: &str = "https://dbfiles.mainnet.iota.cafe/genesis.blob";

pub fn mainnet_poi_dir() -> Result<PathBuf> {
    Ok(iota_config_dir()
        .context("failed to locate the IOTA configuration directory")?
        .join("poi")
        .join("mainnet"))
}

pub async fn load_mainnet_genesis() -> Result<File> {
    let path = mainnet_poi_dir()?.join(IOTA_GENESIS_FILENAME);

    if !path.is_file() {
        let parent = path.parent().context("the genesis cache path must have a parent")?;
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create genesis cache directory '{}'", parent.display()))?;

        println!("Downloading the trusted mainnet genesis blob...");
        let bytes = reqwest::get(MAINNET_GENESIS_URL)
            .await
            .context("failed to download the mainnet genesis blob")?
            .error_for_status()
            .context("the mainnet genesis download returned an error")?
            .bytes()
            .await
            .context("failed to read the downloaded mainnet genesis blob")?;
        fs::write(&path, bytes)
            .with_context(|| format!("failed to cache the mainnet genesis blob at '{}'", path.display()))?;
    }

    File::open(&path).with_context(|| format!("failed to open the mainnet genesis blob '{}'", path.display()))
}
