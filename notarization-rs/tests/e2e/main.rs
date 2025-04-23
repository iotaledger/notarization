// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use async_trait::async_trait;
use identity_storage::StorageSigner;
use identity_storage::JwkMemStore;
use identity_storage::KeyIdMemstore;
use iota_interaction::IotaKeySignature;
use iota_interaction::types::base_types::ObjectID;
use iota_interaction::keytool_signer::KeytoolSigner;
use iota_interaction::IotaClient;
use secret_storage::Signer;
use tokio::sync::OnceCell;
use notarization::{Notarization, NotarizationBuilder};

use crate::common::{MemSigner, TestClient, InitialAccountData};

mod common;
mod dynamic_notarization;

/*
#[async_trait]
impl<S: Signer<IotaKeySignature>> ProductT for NotarizationBuilder<S> {
  async fn new<P>(iota_client: IotaClient, signer: impl Signer<IotaKeySignature>) -> anyhow::Result<NotarizationBuilder<S>> {
    let builder = NotarizationBuilder::new(iota_client).await?
      .signer(signer);
    Ok(builder)
  }

  async fn new_with_pkg_id(iota_client: IotaClient, signer: impl Signer<IotaKeySignature>, iota_product_pkg_id: ObjectID) -> anyhow::Result<Self::FullProduct> {
    let builder = NotarizationBuilder::new_with_pkg_id(iota_client, iota_product_pkg_id).await?
      .signer(signer);
    Ok(builder)
  }

  async fn new_read_only(iota_client: IotaClient) -> anyhow::Result<Self::ReadOnlyProduct> {
    Ok(NotarizationBuilder::new_read_only(iota_client).await?)
  }

  async fn new_read_only_with_pkg_id(iota_client: IotaClient, iota_product_pkg_id: ObjectID) -> anyhow::Result<Self::ReadOnlyProduct> {
    Ok(NotarizationBuilder::new_read_only_with_pkg_id(iota_client, iota_product_pkg_id).await?)
  }

  fn as_full_ref(&self) -> &Self::FullProduct {
    todo!()
  }

  fn as_read_only_ref(&self) -> &Self::ReadOnlyProduct {
    todo!()
  }

  fn as_full_ref_mut(&mut self) -> &mut Self::FullProduct {
    todo!()
  }

  fn as_read_only_ref_mut(&mut self) -> &mut Self::ReadOnlyProduct {
    todo!()
  }
}
*/

async fn new_builder(test_client: &mut TestClient) -> anyhow::Result<NotarizationBuilder<MemSigner>> {
  let user_account_data = test_client.get_funded_client_account().await?;
  let builder = NotarizationBuilder::new(user_account_data.iota_client.clone()).await?
    .signer(user_account_data.signer);
  Ok(builder)
}