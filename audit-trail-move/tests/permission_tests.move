#[test_only]
module audit_trail::permission_tests;

use audit_trail::permission::{Self};
use iota::vec_set;

#[test]
fun test_has_permission_empty_set() {
    let set = permission::empty();
    assert!(vec_set::size(&set) == 0, 0);
}

#[test]
fun test_has_permission_single_permission() {
    let mut set = permission::empty();
    let perm = permission::record_add();
    permission::add(&mut set, perm);
    
    assert!(permission::has_permission(&set, &perm), 0);
}

#[test]
fun test_has_permission_not_in_set() {
    let mut set = permission::empty();
    permission::add(&mut set, permission::record_add());
    
    let perm = permission::record_delete();
    assert!(!permission::has_permission(&set, &perm), 0);
}

#[test]
fun test_has_permission_multiple_permission() {
    let mut set = permission::empty();
    permission::add(&mut set, permission::record_add());
    permission::add(&mut set, permission::record_delete());
    permission::add(&mut set, permission::audit_trail_delete());
    
    assert!(permission::has_permission(&set, &permission::record_add()), 0);
    assert!(permission::has_permission(&set, &permission::record_delete()), 0);
    assert!(permission::has_permission(&set, &permission::audit_trail_delete()), 0);
    assert!(!permission::has_permission(&set, &permission::record_correct()), 0);
}

#[test]
fun test_has_permission_from_vec() {
    let perms = vector[
        permission::record_add(),
        permission::record_delete(),
        permission::meta_data_update(),
    ];
    let set = permission::from_vec(perms);
    
    assert!(permission::has_permission(&set, &permission::record_add()), 0);
    assert!(permission::has_permission(&set, &permission::record_delete()), 0);
    assert!(permission::has_permission(&set, &permission::meta_data_update()), 0);
    assert!(!permission::has_permission(&set, &permission::audit_trail_delete()), 0);
}

#[test]
fun test_from_vec_empty() {
    let perms = vector[];
    let set = permission::from_vec(perms);
    
    assert!(vec_set::size(&set) == 0, 0);
}

#[test]
fun test_from_vec_single_permission() {
    let perms = vector[permission::record_add()];
    let set = permission::from_vec(perms);
    
    assert!(vec_set::size(&set) == 1, 0);
    assert!(permission::has_permission(&set, &permission::record_add()), 0);
}

#[test]
fun test_from_vec_multiple_permission() {
    let perms = vector[
        permission::record_add(),
        permission::record_delete(),
        permission::audit_trail_delete(),
    ];
    let set = permission::from_vec(perms);
    
    assert!(vec_set::size(&set) == 3, 0);
    assert!(permission::has_permission(&set, &permission::record_add()), 0);
    assert!(permission::has_permission(&set, &permission::record_delete()), 0);
    assert!(permission::has_permission(&set, &permission::audit_trail_delete()), 0);
    assert!(!permission::has_permission(&set, &permission::record_correct()), 0);
}

#[test]
fun test_metadata_admin_permissions() {
    let perms = permission::metadata_admin_permissions();
    
    assert!(permission::has_permission(&perms, &permission::meta_data_update()), 0);
    assert!(permission::has_permission(&perms, &permission::meta_data_delete()), 0);
    assert!(iota::vec_set::size(&perms) == 2, 0);
}

#[test]
#[expected_failure(abort_code = vec_set::EKeyAlreadyExists)]
fun test_from_vec_duplicate_permission() {
    // VecSet should throw error EKeyAlreadyExists on duplicate insertions
    let perms = vector[
        permission::record_add(),
        permission::record_delete(),
        permission::record_add(), // duplicate
    ];
    let set = permission::from_vec(perms);
    // The following line should not be reached due to the expected failure
    assert!(vec_set::size(&set) == 2, 0);
}