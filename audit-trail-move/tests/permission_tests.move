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
    let perm = permission::add_record();
    permission::add(&mut set, perm);
    
    assert!(permission::has_permission(&set, &perm), 0);
}

#[test]
fun test_has_permission_not_in_set() {
    let mut set = permission::empty();
    permission::add(&mut set, permission::add_record());
    
    let perm = permission::delete_record();
    assert!(!permission::has_permission(&set, &perm), 0);
}

#[test]
fun test_has_permission_multiple_permission() {
    let mut set = permission::empty();
    permission::add(&mut set, permission::add_record());
    permission::add(&mut set, permission::delete_record());
    permission::add(&mut set, permission::delete_audit_trail());
    
    assert!(permission::has_permission(&set, &permission::add_record()), 0);
    assert!(permission::has_permission(&set, &permission::delete_record()), 0);
    assert!(permission::has_permission(&set, &permission::delete_audit_trail()), 0);
    assert!(!permission::has_permission(&set, &permission::correct_record()), 0);
}

#[test]
fun test_has_permission_from_vec() {
    let perms = vector[
        permission::add_record(),
        permission::delete_record(),
        permission::update_metadata(),
    ];
    let set = permission::from_vec(perms);
    
    assert!(permission::has_permission(&set, &permission::add_record()), 0);
    assert!(permission::has_permission(&set, &permission::delete_record()), 0);
    assert!(permission::has_permission(&set, &permission::update_metadata()), 0);
    assert!(!permission::has_permission(&set, &permission::delete_audit_trail()), 0);
}

#[test]
fun test_from_vec_empty() {
    let perms = vector[];
    let set = permission::from_vec(perms);
    
    assert!(vec_set::size(&set) == 0, 0);
}

#[test]
fun test_from_vec_single_permission() {
    let perms = vector[permission::add_record()];
    let set = permission::from_vec(perms);
    
    assert!(vec_set::size(&set) == 1, 0);
    assert!(permission::has_permission(&set, &permission::add_record()), 0);
}

#[test]
fun test_from_vec_multiple_permission() {
    let perms = vector[
        permission::add_record(),
        permission::delete_record(),
        permission::delete_audit_trail(),
    ];
    let set = permission::from_vec(perms);
    
    assert!(vec_set::size(&set) == 3, 0);
    assert!(permission::has_permission(&set, &permission::add_record()), 0);
    assert!(permission::has_permission(&set, &permission::delete_record()), 0);
    assert!(permission::has_permission(&set, &permission::delete_audit_trail()), 0);
    assert!(!permission::has_permission(&set, &permission::correct_record()), 0);
}

#[test]
fun test_metadata_admin_permissions() {
    let perms = permission::metadata_admin_permissions();
    
    assert!(permission::has_permission(&perms, &permission::update_metadata()), 0);
    assert!(permission::has_permission(&perms, &permission::delete_metadata()), 0);
    assert!(iota::vec_set::size(&perms) == 2, 0);
}

#[test]
#[expected_failure(abort_code = vec_set::EKeyAlreadyExists)]
fun test_from_vec_duplicate_permission() {
    // VecSet should throw error EKeyAlreadyExists on duplicate insertions
    let perms = vector[
        permission::add_record(),
        permission::delete_record(),
        permission::add_record(), // duplicate
    ];
    let set = permission::from_vec(perms);
    // The following line should not be reached due to the expected failure
    assert!(vec_set::size(&set) == 2, 0);
}