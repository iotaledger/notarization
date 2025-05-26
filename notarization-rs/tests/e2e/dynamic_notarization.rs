// Copyright 2020-2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::client::get_funded_test_client;
use anyhow::anyhow;
use anyhow::Context;
use iota_interaction::IotaClientTrait;
use iota_sdk::rpc_types::{IotaData, IotaObjectDataOptions};
use notarization::core::notarization::OnChainNotarization;
use notarization::core::state::{Data, State};
use notarization::core::NotarizationMethod;
use product_common::core_client::CoreClientReadOnly;
use serde_json::Value;
#[tokio::test]
async fn create_simple_dynamic_notarization_works() -> anyhow::Result<()> {
    let mut test_client = get_funded_test_client().await?;

    let notarization: OnChainNotarization = test_client
        .client_adapter()
        .read_api()
        .get_object_with_options(
            "0xc96a9710a18015fb8d9f43f80f2e65e19ae68c452b25093ef2697a7ebce116cc"
                .parse()
                .unwrap(),
            IotaObjectDataOptions::bcs_lossless(),
        )
        .await
        .context("lookup request failed")
        .and_then(|res| res.data.context("missing data in response"))
        .and_then(|data| data.bcs.context("missing object content in data"))
        .and_then(|content| content.try_into_move().context("not a move object"))
        .and_then(|obj| {
            obj.deserialize()
                .map_err(|err| anyhow!("failed to deserialize move object; {err}"))
        })
        .context("failed to get object by id")
        .unwrap();

    println!("notarization json: {:?}", notarization);

    // let notarization: OnChainNotarization = serde_json::from_value(notarization).unwrap();

    // println!("notarization: {:?}", notarization);

    // // Create a dynamic notarization
    // let onchain_notarization = test_client
    //     .create_dynamic_notarization()
    //     .with_state(State::from_string("test".to_string(), None))
    //     .with_immutable_description("Test Notarization".to_string())
    //     .finish()
    //     .build_and_execute(&mut test_client)
    //     .await?
    //     .output;

    // println!("onchain_notarization: {:?}", onchain_notarization);

    // assert_eq!(onchain_notarization.state.data, IotaData::Text("test".to_string()));
    // assert_eq!(
    //     onchain_notarization.immutable_metadata.description,
    //     Some("Test Notarization".to_string())
    // );
    // assert_eq!(onchain_notarization.immutable_metadata.locking, None);
    // assert_eq!(onchain_notarization.updateable_metadata, None);
    // assert_eq!(onchain_notarization.last_state_change_at, 0);
    // assert_eq!(onchain_notarization.state_version_count, 0);
    // assert_eq!(onchain_notarization.method, NotarizationMethod::Dynamic);
    Ok(())
}
