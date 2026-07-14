//! Integration tests for the knowledge base module.

use crate::kb::{Ent::*, Tuple, KB};
use std::error::Error;
use std::result;

pub type Result<T> = result::Result<T, Box<dyn Error>>;

/// Integration tests for the knowledge base module.

#[test]
fn test_kb_tuple_storage() -> Result<()> {
    // Create a new KB instance
    let kb = KB::new("test.db")?;

    // Create a tuple
    let tuple = Tuple::new("Alice", "knows", "Bob", 0.9);

    // Store the tuple
    kb.store_tuple(&tuple)?;

    // Retrieve the tuple
    let retrieved = kb.retrieve_tuple("Alice", "knows", "Bob")?;
    assert!(retrieved.is_some());

    Ok(())
}
