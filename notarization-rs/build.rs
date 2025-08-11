use std::path::PathBuf;
use product_common::move_history_manager::MoveHistoryManager;

fn main() {
    let move_lock_path = "../notarization-move/Move.lock";
    println!("[build.rs] move_lock_path: {move_lock_path}");
    let move_history_path = "../notarization-move/Move.history.json";
    println!("[build.rs] move_history_path: {move_history_path}");

    MoveHistoryManager::new(
        &PathBuf::from(move_lock_path),
        &PathBuf::from(move_history_path),
        // Use `Some(vec![])` instead of `None`, if you don't want to ignore `localnet`
        None,
    ).manage_history_file(|message| { println!("[build.rs] {}", message); })
        .expect("Successfully managed Move history file");

    // Tell Cargo to rerun this build script if the Move.lock file changes.
    println!("cargo::rerun-if-changed={move_lock_path}");
}