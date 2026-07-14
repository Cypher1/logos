//! Knowledge Base module for Logos, using redb as an embedded database.
//!
//! This module provides the core data structures and functionality for
//! storing and retrieving knowledge base tuples using redb.

#[cfg(test)]
mod tests;
pub mod tuple;

pub use tuple::Ent;
pub use tuple::Tuple;

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use std::error::Error;
use std::result;

const TUPLES_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("tuples");
const PREDICATE_TABLE: TableDefinition<&str, u64> = TableDefinition::new("tuples_by_predicate");
const SUBJECT_TABLE: TableDefinition<&str, u64> = TableDefinition::new("tuples_by_subject");
const OBJECT_TABLE: TableDefinition<&str, u64> = TableDefinition::new("tuples_by_object");

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
        let id = tuple.id();
        let value = serde_json::to_vec(tuple)?;

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(TUPLES_TABLE)?;
            table.insert(id, value.as_slice())?;

            // Index by predicate
            let mut pred_table = write_txn.open_table(PREDICATE_TABLE)?;
            pred_table.insert(format!("pred::{}", tuple.predicate).as_str(), &id)?;

            // Index by subject
            let mut sub_table = write_txn.open_table(SUBJECT_TABLE)?;
            sub_table.insert(format!("sub::{}", tuple.subject).as_str(), &id)?;

            // Index by object
            let mut obj_table = write_txn.open_table(OBJECT_TABLE)?;
            obj_table.insert(format!("obj::{}", tuple.object).as_str(), &id)?;
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
        let tuple = Tuple::new(subject.into(), predicate.into(), object.into(), 0.5);
        let id = tuple.id();
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(TUPLES_TABLE)?;
        if let Some(guard) = table.get(&id)? {
            let tuple: Tuple = serde_json::from_slice(guard.value())?;
            return Ok(Some(tuple));
        }

        Ok(None)
    }

    /// Retrieves all tuples from the knowledge base.
    pub fn get_all_tuples(&self) -> result::Result<Vec<Tuple>, Box<dyn Error>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(TUPLES_TABLE)?;
        let mut tuples = Vec::new();

        let mut iter = table.iter()?;
        for res in iter {
            let (_key_guard, val_guard) = res?;
            let tuple: Tuple = serde_json::from_slice(val_guard.value())?;
            tuples.push(tuple);
        }

        Ok(tuples)
    }
}
