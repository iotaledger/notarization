// Copyright 2020-2026 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::error::Error;
use iota_interaction::ident_str;
use iota_interaction::types::Identifier;
use iota_interaction::types::TypeTag;
use iota_interaction::types::base_types::{IotaAddress, ObjectID};
use iota_interaction::types::programmable_transaction_builder::ProgrammableTransactionBuilder as Ptb;
use iota_interaction::types::transaction::Argument;
use iota_interaction::types::transaction::{Command, ObjectArg, ProgrammableTransaction};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::str::FromStr;

/// Permission enum matching the Move permission module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Permission {
    DeleteAuditTrail,
    AddRecord,
    DeleteRecord,
    CorrectRecord,
    UpdateLockingConfig,
    UpdateLockingConfigForDeleteRecord,
    UpdateLockingConfigForDeleteTrail,
    AddRoles,
    UpdateRoles,
    DeleteRoles,
    AddCapabilities,
    RevokeCapabilities,
    UpdateMetadata,
    DeleteMetadata,
}

impl Permission {
    /// Returns the Move constructor function name for this permission variant.
    pub(crate) fn function_name(&self) -> &'static str {
        match self {
            Self::DeleteAuditTrail => "delete_audit_trail",
            Self::AddRecord => "add_record",
            Self::DeleteRecord => "delete_record",
            Self::CorrectRecord => "correct_record",
            Self::UpdateLockingConfig => "update_locking_config",
            Self::UpdateLockingConfigForDeleteRecord => "update_locking_config_for_delete_record",
            Self::UpdateLockingConfigForDeleteTrail => "update_locking_config_for_delete_trail",
            Self::AddRoles => "add_roles",
            Self::UpdateRoles => "update_roles",
            Self::DeleteRoles => "delete_roles",
            Self::AddCapabilities => "add_capabilities",
            Self::RevokeCapabilities => "revoke_capabilities",
            Self::UpdateMetadata => "update_metadata",
            Self::DeleteMetadata => "delete_metadata",
        }
    }

    pub(crate) fn tag(package_id: ObjectID) -> TypeTag {
        TypeTag::from_str(&format!("{package_id}::permission::Permission")).expect("invalid TypeTag for Permission")
    }

    pub(in crate::core) fn to_ptb(&self, ptb: &mut Ptb, package_id: ObjectID) -> Result<Argument, Error> {
        let function = Identifier::from_str(self.function_name())
            .map_err(|e| Error::InvalidArgument(format!("Failed to create identifier for function: {e}")))?;

        Ok(ptb.programmable_move_call(package_id, ident_str!("permission").into(), function, vec![], vec![]))
    }
}

/// Convenience wrapper for permission sets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PermissionSet {
    pub permissions: HashSet<Permission>,
}

impl PermissionSet {
    pub(crate) fn to_move_vec(&self, package_id: ObjectID, ptb: &mut Ptb) -> Result<Argument, Error> {
        let permission_type = Permission::tag(package_id);
        let permission_args: Vec<_> = self
            .permissions
            .iter()
            .map(|permission| permission.to_ptb(ptb, package_id))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ptb.command(Command::MakeMoveVec(Some(permission_type.into()), permission_args)))
    }
    pub fn admin_permissions() -> Self {
        Self {
            permissions: HashSet::from([
                Permission::DeleteAuditTrail,
                Permission::AddCapabilities,
                Permission::RevokeCapabilities,
                Permission::AddRoles,
                Permission::UpdateRoles,
                Permission::DeleteRoles,
            ]),
        }
    }

    pub fn record_admin_permissions() -> Self {
        Self {
            permissions: HashSet::from([
                Permission::AddRecord,
                Permission::DeleteRecord,
                Permission::CorrectRecord,
            ]),
        }
    }

    pub fn locking_admin_permissions() -> Self {
        Self {
            permissions: HashSet::from([
                Permission::UpdateLockingConfig,
                Permission::UpdateLockingConfigForDeleteTrail,
                Permission::UpdateLockingConfigForDeleteRecord,
            ]),
        }
    }

    pub fn role_admin_permissions() -> Self {
        Self {
            permissions: HashSet::from([Permission::AddRoles, Permission::UpdateRoles, Permission::DeleteRoles]),
        }
    }

    pub fn cap_admin_permissions() -> Self {
        Self {
            permissions: HashSet::from_iter(vec![Permission::AddCapabilities, Permission::RevokeCapabilities]),
        }
    }

    pub fn metadata_admin_permissions() -> Self {
        Self {
            permissions: HashSet::from_iter(vec![Permission::UpdateMetadata, Permission::DeleteMetadata]),
        }
    }
}
