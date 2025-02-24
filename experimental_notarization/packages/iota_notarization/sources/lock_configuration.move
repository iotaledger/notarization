module iota_notarization::lock_configuration {
    use iota_notarization::timelock_unlock_condition::infinite_lock;

    /// Invalid lock order (delete lock period must be greater than update lock period)
    const EInvalidLockOrder: u64 = 6;

    /// Configuration for notarization locks
    ///
    /// ### Warning
    ///
    /// If you set any of the values to 0, the notarization can not be
    /// updated or deleted, depending on the value set to 0.
    public struct LockConfiguration has copy, drop, store {
        /// Time until state updates are locked (0 for infinite lock)
        update_lock_period: u32,
        /// Time until deletion is locked (0 for infinite lock)
        delete_lock_period: u32,
    }

    // ===== Lock Configuration Helpers =====
    /// Create a new lock configuration
    public fun new_lock_configuration(update_lock_period: u32, delete_lock_period: u32): LockConfiguration {
        assert!(delete_lock_period > update_lock_period, EInvalidLockOrder);

        LockConfiguration {
            update_lock_period,
            delete_lock_period,
        }
    }

    /// Create a configuration with infinite locks for both update and delete
    public fun infinite_locks(): LockConfiguration {
        LockConfiguration {
            update_lock_period: infinite_lock(),
            delete_lock_period: infinite_lock(),
        }
    }
    // ===== Getters =====
    /// Get the update lock period
    /// Returns 0 for infinite lock
    public fun update_lock_period(self: &LockConfiguration): u32 {
        self.update_lock_period
    }

    /// Get the delete lock period
    public fun delete_lock_period(self: &LockConfiguration): u32 {
        self.delete_lock_period
    }
}