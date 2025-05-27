// // Copyright 2020-2025 IOTA Stiftung
// // SPDX-License-Identifier: Apache-2.0

// use crate::common::get_funded_test_client;
// use crate::new_builder;

// #[tokio::test]
// async fn create_dynamic_notarization() -> anyhow::Result<()>{
//   let mut test_client = get_funded_test_client().await?;
//   let builder = new_builder(&mut test_client).await?;
//   let dyn_notarization = builder.create_dynamic().await?;

//  // assert!(dyn_notarization.did_document().metadata.deactivated == Some(true));

//   Ok(())
// }