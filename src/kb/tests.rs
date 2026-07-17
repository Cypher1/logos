//! Integration tests for the knowledge base module.
//!
//! These tests ensure that tuples can be correctly stored and retrieved from the KB,
//! checking both main storage and all secondary indexes (predicate, subject, object).

use crate::kb::{KB, Tuple};
use std::error::Error;
use std::fs;
use pretty_assertions::assert_eq;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// Helper to create a clean KB instance for testing.
fn setup_kb(name: &str) -> Result<(KB, String)> {
    let path = format!("test_{}.db", name);
    let _ = fs::remove_file(&path);
    let kb = KB::new(&path)?;
    Ok((kb, path))
}

/// Helper to cleanup a test database file.
fn teardown_db(path: &str) {
    let _ = fs::remove_file(path);
}

#[test]
fn test_kb_tuple_storage_and_indexes() -> Result<()> {
    let (kb, path) = setup_kb("storage")?;

    // Test data
    let tuple = Tuple::new("Alice", "knows", "Bob", 0.9);
    kb.store_tuple(&tuple)?;

    // 1. Verify main storage retrieval
    let retrieved = kb.retrieve_tuple("Alice", "knows", "Bob")?;
    let r = retrieved;
    assert_eq!(r.subject, "Alice".into());
    assert_eq!(r.predicate, "knows".into());
    assert_eq!(r.object, "Bob".into());
    assert!(r.confidence > 0.8999 && r.confidence < 0.9001);

    // 2. Verify all_tuples retrieval
    let all = kb.get_all_tuples()?;
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].subject, "Alice".into());

    // 3. Multiple entries & index integrity check (sequential stores)
    kb.store_tuple(&Tuple::new("Charlie", "likes", "Dave", 0.8))?;
    kb.store_tuple(&Tuple::new("Eve", "owns", "Treasure", 1.0))?;

    let all_many = kb.get_all_tuples()?;
    assert_eq!(all_many.len(), 3);

    // Test non-existent
    let missing = kb.retrieve_tuple("Unknown", "does", "Nothing")?;
    assert!(missing.confidence > 0.4999 && missing.confidence < 0.5001, "Confidence {conf}", conf=missing.confidence);

    teardown_db(&path);
    Ok(())
}

#[test]
fn test_get_all_tuples_empty() -> Result<()> {
    let (kb, path) = setup_kb("empty")?;
    let tuples = kb.get_all_tuples()?;
    assert!(tuples.is_empty());

    teardown_db(&path);
    Ok(())
}

#[test]
fn test_retrieve_by_subject() -> Result<()> {
    let (kb, path) = setup_kb("subject")?;
    let tuple1 = Tuple::new("Alice", "knows", "Bob", 0.9);
    let tuple2 = Tuple::new("Alice", "likes", "Charlie", 0.8);
    let tuple3 = Tuple::new("Charlie", "knows", "Dave", 0.7);
    kb.store_tuple(&tuple1)?;
    kb.store_tuple(&tuple2)?;
    kb.store_tuple(&tuple3)?;

    let mut results = kb.retrieve_by_subject("Alice")?;
    assert_eq!(results.len(), 2);
    assert!(results.contains(&tuple1.id()));
    assert!(results.contains(&tuple2.id()));
    assert!(!results.contains(&tuple3.id()));

    teardown_db(&path);
    Ok(())
}

#[test]
fn test_retrieve_by_predicate() -> Result<()> {
    let (kb, path) = setup_kb("predicate")?;
    let tuple1 = Tuple::new("Alice", "knows", "Bob", 0.9);
    let tuple2 = Tuple::new("Charlie", "knows", "Dave", 0.7);
    let tuple3 = Tuple::new("Eve", "owns", "Treasure", 1.0);
    kb.store_tuple(&tuple1)?;
    kb.store_tuple(&tuple2)?;
    kb.store_tuple(&tuple3)?;

    let mut results = kb.retrieve_by_predicate("knows")?;
    assert_eq!(results.len(), 2);
    assert!(results.contains(&tuple1.id()));
    assert!(results.contains(&tuple2.id()));
    assert!(!results.contains(&tuple3.id()));

    teardown_db(&path);
    Ok(())
}

#[test]
fn test_retrieve_by_object() -> Result<()> {
    let (kb, path) = setup_kb("object")?;
    let tuple1 = Tuple::new("Alice", "knows", "Bob", 0.9);
    let tuple2 = Tuple::new("Charlie", "likes", "Bob", 0.8);
    let tuple3 = Tuple::new("Eve", "owns", "Treasure", 1.0);
    kb.store_tuple(&tuple1)?;
    kb.store_tuple(&tuple2)?;
    kb.store_tuple(&tuple3)?;

    let mut results = kb.retrieve_by_object("Bob")?;
    assert_eq!(results.len(), 2);
    assert!(results.contains(&tuple1.id()));
    assert!(results.contains(&tuple2.id()));
    assert!(!results.contains(&tuple3.id()));

    teardown_db(&path);
    Ok(())
}
