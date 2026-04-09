// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use iota_interaction::move_types::language_storage::StructTag;
use iota_interaction::rpc_types::{
    IotaObjectDataFilter, IotaObjectDataOptions, IotaObjectResponseQuery, IotaParsedData,
};
use iota_interaction::types::TypeTag;
use iota_interaction::types::base_types::{IotaAddress, ObjectID, ObjectRef};
use iota_interaction::types::dynamic_field::DynamicFieldName;
use iota_interaction::types::id::ID;
use iota_interaction::{IotaClientTrait, OptionalSync};
use product_common::core_client::CoreClientReadOnly;

use super::{linked_table, tx};
use crate::core::types::{Capability, OnChainAuditTrail, Permission};
use crate::error::Error;

/// Finds an owned capability that grants `permission` on `trail_id`.
///
/// This is the standard lookup path used by most trail operations. It derives
/// the set of role names that grant the requested permission from the current
/// on-chain trail state, then delegates the actual owned-object scan to
/// [`find_owned_capability`]. The selected capability is returned as an
/// [`ObjectRef`] because transaction construction needs the live object
/// reference, not just the parsed capability payload.
pub(crate) async fn find_capable_cap<C>(
    client: &C,
    owner: IotaAddress,
    trail_id: ObjectID,
    trail: &OnChainAuditTrail,
    permission: Permission,
) -> Result<ObjectRef, Error>
where
    C: CoreClientReadOnly + OptionalSync,
{
    let valid_roles: HashSet<String> = trail
        .roles
        .roles
        .iter()
        .filter(|(_, role)| role.permissions.contains(&permission))
        .map(|(name, _)| name.clone())
        .collect();

    let cap = find_owned_capability(client, owner, trail, |cap| {
        cap.matches_target_and_role(trail_id, &valid_roles)
    })
    .await?
    .ok_or_else(|| {
        Error::InvalidArgument(format!(
            "no capability with {:?} permission found for owner {owner} and trail {trail_id}",
            permission
        ))
    })?;

    let object_id = *cap.id.object_id();
    tx::get_object_ref_by_id(client, &object_id).await
}

/// Finds the first owned capability that survives common local filtering.
///
/// This helper is the generic capability scanner used by the more specific
/// permission-based and tag-aware lookup functions below. It handles:
/// - fetching owned capability objects page by page,
/// - excluding revoked capability IDs recorded on the trail, and
/// - enforcing any `issued_to` address restriction locally.
///
/// The caller supplies the remaining policy via `predicate`, typically matching
/// the target trail and one or more allowed role names.
pub(crate) async fn find_owned_capability<C, P>(
    client: &C,
    owner: IotaAddress,
    trail: &OnChainAuditTrail,
    predicate: P,
) -> Result<Option<Capability>, Error>
where
    C: CoreClientReadOnly + OptionalSync,
    P: Fn(&Capability) -> bool + Send,
{
    let revoked_capability_ids = revoked_capability_ids(client, trail).await?;
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let tf_components_package_id = client
        .tf_components_package_id()
        .expect("TfComponents package ID should be present for audit trail clients");
    let capability_struct_tag: StructTag = Capability::type_tag(tf_components_package_id)
        .to_string()
        .parse()
        .expect("capability type tag is a valid struct tag");
    let query = IotaObjectResponseQuery::new(
        Some(IotaObjectDataFilter::StructType(capability_struct_tag)),
        Some(IotaObjectDataOptions::default().with_content()),
    );

    let mut cursor = None;
    loop {
        let mut page = client
            .client_adapter()
            .read_api()
            .get_owned_objects(owner, Some(query.clone()), cursor, Some(25))
            .await
            .map_err(|e| Error::RpcError(e.to_string()))?;

        let maybe_cap = std::mem::take(&mut page.data)
            .into_iter()
            .filter_map(|res| res.data)
            .filter_map(|data| data.content)
            .filter_map(|obj_data| {
                let IotaParsedData::MoveObject(move_object) = obj_data else {
                    unreachable!()
                };
                serde_json::from_value(move_object.fields.to_json_value()).ok()
            })
            .find(|cap| capability_matches(cap, owner, now_ms, &revoked_capability_ids, &predicate));
        cursor = page.next_cursor;

        if maybe_cap.is_some() {
            return Ok(maybe_cap);
        }
        if !page.has_next_page {
            break;
        }
    }

    Ok(None)
}

/// Loads the current revoked-capability denylist from the trail's linked table.
///
/// The resulting set is used during local capability selection so revoked
/// capabilities are ignored before transaction construction.
async fn revoked_capability_ids<C>(client: &C, trail: &OnChainAuditTrail) -> Result<HashSet<ObjectID>, Error>
where
    C: CoreClientReadOnly + OptionalSync,
{
    let table = &trail.roles.revoked_capabilities;
    let expected = table.size as usize;
    let mut cursor = table.head;
    let mut keys = HashSet::with_capacity(expected);

    while let Some(key) = cursor {
        if !keys.insert(key) {
            return Err(Error::UnexpectedApiResponse(format!(
                "cycle detected while traversing linked-table {table_id}; repeated key {key}",
                table_id = table.id
            )));
        }

        let node = linked_table::fetch_node::<_, ObjectID, u64>(
            client,
            table.id,
            DynamicFieldName {
                type_: TypeTag::Struct(Box::new(ID::type_())),
                value: serde_json::Value::String(IotaAddress::from(key).to_string()),
            },
        )
        .await?;
        cursor = node.next;
    }

    if keys.len() != expected {
        return Err(Error::UnexpectedApiResponse(format!(
            "linked-table traversal mismatch; expected {expected} entries, got {}",
            keys.len()
        )));
    }

    Ok(keys)
}

/// Applies the shared local capability filters.
///
/// A capability is considered usable locally when:
/// - the caller-specific predicate matches,
/// - the capability ID is not present in the trail's revoked-capability set, and
/// - any `issued_to` restriction matches the current owner address, and
/// - the current local time falls within the capability's validity window.
fn capability_matches<P>(
    cap: &Capability,
    owner: IotaAddress,
    now_ms: u64,
    revoked_capability_ids: &HashSet<ObjectID>,
    predicate: &P,
) -> bool
where
    P: Fn(&Capability) -> bool,
{
    predicate(cap)
        && !revoked_capability_ids.contains(cap.id.object_id())
        && cap.issued_to.map(|issued_to| issued_to == owner).unwrap_or(true)
        && cap.valid_from.is_none_or(|valid_from| now_ms >= valid_from)
        && cap.valid_until.is_none_or(|valid_until| now_ms <= valid_until)
}

/// Finds an owned capability for adding a tagged record.
///
/// Tagged writes have stricter lookup rules than ordinary permission-based
/// operations: the selected role must grant `AddRecord` and its configured
/// `RoleTags` must allow the requested record tag.
pub(crate) async fn find_capable_cap_for_tag<C>(
    client: &C,
    owner: IotaAddress,
    trail_id: ObjectID,
    trail: &OnChainAuditTrail,
    tag: &str,
) -> Result<iota_interaction::types::base_types::ObjectRef, Error>
where
    C: CoreClientReadOnly + OptionalSync,
{
    let valid_roles = trail
        .roles
        .roles
        .iter()
        .filter(|(_, role)| {
            role.permissions.contains(&Permission::AddRecord)
                && role.data.as_ref().is_some_and(|record_tags| record_tags.allows(tag))
        })
        .map(|(name, _)| name.clone())
        .collect::<std::collections::HashSet<_>>();

    let cap = find_owned_capability(client, owner, trail, |cap| {
        cap.target_key == trail_id && valid_roles.contains(&cap.role)
    })
    .await?
    .ok_or_else(|| {
        Error::InvalidArgument(format!(
            "no capability with {:?} permission and record tag '{tag}' found for owner {owner} and trail {trail_id}",
            Permission::AddRecord
        ))
    })?;

    let object_id = *cap.id.object_id();
    tx::get_object_ref_by_id(client, &object_id).await
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use iota_interaction::types::base_types::{IotaAddress, ObjectID, dbg_object_id};
    use iota_interaction::types::id::UID;

    use super::capability_matches;
    use crate::core::types::Capability;

    #[test]
    fn capability_matches_skips_revoked_caps() {
        let owner = IotaAddress::random_for_testing_only();
        let trail_id = dbg_object_id(1);
        let revoked_cap_id = dbg_object_id(2);
        let valid_cap_id = dbg_object_id(3);
        let valid_roles = HashSet::from(["Writer".to_string()]);
        let revoked_ids = HashSet::from([revoked_cap_id]);

        let revoked_cap = make_capability(revoked_cap_id, trail_id, "Writer", None);
        let valid_cap = make_capability(valid_cap_id, trail_id, "Writer", None);

        assert!(!capability_matches(&revoked_cap, owner, 0, &revoked_ids, &|cap| cap
            .matches_target_and_role(trail_id, &valid_roles)));
        assert!(capability_matches(&valid_cap, owner, 0, &revoked_ids, &|cap| cap
            .matches_target_and_role(trail_id, &valid_roles)));
    }

    #[test]
    fn capability_matches_skips_issued_to_mismatch() {
        let owner = IotaAddress::random_for_testing_only();
        let other_owner = IotaAddress::random_for_testing_only();
        let trail_id = dbg_object_id(4);
        let valid_roles = HashSet::from(["Writer".to_string()]);
        let cap = make_capability(dbg_object_id(5), trail_id, "Writer", Some(other_owner));

        assert!(!capability_matches(&cap, owner, 0, &HashSet::new(), &|candidate| {
            candidate.matches_target_and_role(trail_id, &valid_roles)
        }));
    }

    #[test]
    fn capability_matches_skips_caps_before_valid_from() {
        let owner = IotaAddress::random_for_testing_only();
        let trail_id = dbg_object_id(6);
        let valid_roles = HashSet::from(["Writer".to_string()]);
        let mut cap = make_capability(dbg_object_id(7), trail_id, "Writer", None);
        cap.valid_from = Some(2_000);

        assert!(!capability_matches(&cap, owner, 1_999, &HashSet::new(), &|candidate| {
            candidate.matches_target_and_role(trail_id, &valid_roles)
        }));
        assert!(capability_matches(&cap, owner, 2_000, &HashSet::new(), &|candidate| {
            candidate.matches_target_and_role(trail_id, &valid_roles)
        }));
    }

    #[test]
    fn capability_matches_skips_caps_after_valid_until() {
        let owner = IotaAddress::random_for_testing_only();
        let trail_id = dbg_object_id(8);
        let valid_roles = HashSet::from(["Writer".to_string()]);
        let mut cap = make_capability(dbg_object_id(9), trail_id, "Writer", None);
        cap.valid_until = Some(2_000);

        assert!(capability_matches(&cap, owner, 2_000, &HashSet::new(), &|candidate| {
            candidate.matches_target_and_role(trail_id, &valid_roles)
        }));
        assert!(!capability_matches(&cap, owner, 2_001, &HashSet::new(), &|candidate| {
            candidate.matches_target_and_role(trail_id, &valid_roles)
        }));
    }

    fn make_capability(id: ObjectID, trail_id: ObjectID, role: &str, issued_to: Option<IotaAddress>) -> Capability {
        Capability {
            id: UID::new(id),
            target_key: trail_id,
            role: role.to_string(),
            issued_to,
            valid_from: None,
            valid_until: None,
        }
    }
}
