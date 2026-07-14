//! Knowledge Base module for Logos, using redb as an embedded database.
//!
//! This module provides the core data structures and functionality for
//! storing and retrieving knowledge base tuples using redb.

pub mod tuple;
#[cfg(test)]
mod tests;

pub use tuple::Tuple;
pub use tuple::Ent;

use redb::{Database, TableDefinition, ReadableDatabase, ReadableTable};
use std::error::Error;
use std::result;

const TUPLES_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("tuples");
const PREDICATE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("tuples_by_predicate");
const SUBJECT_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("tuples_by_subject");
const OBJECT_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("tuples_by_object");

/// A simple wrapper around redb's database to manage tuples.
pub struct KB {
    db: Database,
}

impl KB {
    /// Creates a new KnowledgeBase instance.
    pub fn new(path: impl AsRef<std::path::Path>) -> result::Result<Self, Box<dyn Error>> {
        let db = Database::create(path)?;
        
        // Initialize the table by opening a write transaction and committing it
        let write_txn = db.begin_write()?;
        {
            let _table = write_txn.open_table(TUPLES_TABLE)?;
        }
        write_txn.commit()?;
        
        Ok(KB { db })
    }

    /// Inserts a tuple into the knowledge base.
    pub fn store_tuple(&self, tuple: &Tuple) -> result::Result<(), Box<dyn Error>> {
        let key = format!("{}::{}::{}", tuple.subject, tuple.predicate, tuple.object);
        let value = serde_json::to_vec(tuple)?;
        
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(TUPLES_TABLE)?;
            table.insert(key.as_str(), value.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Retrieves a tuple from the knowledge base by its key.
    pub fn retrieve_tuple(
        &self,
        subject: impl Into<Ent>,
        predicate: impl Into<Ent>,
        object: impl Into<Ent>,
    ) -> result::Result<Option<Tuple>, Box<dyn Error>> {
        let key = format!("{}::{}::{}", subject.into(), predicate.into(), object.into());
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(TUPLES_TABLE)?;
        if let Some(guard) = table.get(key.as_str())? {
            let tuple: Tuple = serde_json::from_slice(guard.value())?;
            Ok(Some(tuple))
        } else {
            Ok(None)
        }
    }

    /// Retrieves all tuples from the knowledge base.
    pub fn get_all_tuples(&self) -> result::Result<Vec<Tuple>, Box<dyn Error>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(TUPLES_TABLE)?;
        let mut tuples = Vec::new();
        
        let mut iter = table.iter()?;
        while let Some(res) = iter.next() {
            let (_key_guard, val_guard) = res?;
            let tuple: Tuple = serde_json::from_slice(val_guard.value())?;
            tuples.push(tuple);
        }
        
        Ok(tuples)
    }

    /// Shuts down the database.
    pub fn shutdown(self) -> result::Result<(), Box<dyn Error>> {
        Ok(())
    }
}
