// Copyright (c) 2025 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

/// Phantom type marker for audit trail authorization.
///
/// Used as the phantom type parameter P in `OperationCap<P>` and
/// `AccessControllerBridge<P>` to provide compile-time type safety
/// between different component types.
module audit_trail::marker;

public struct AuditTrailPerm has drop {}
