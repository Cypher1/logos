//! Knowledge Base module for Logos, using redb as an embedded database.
//!
//! This module provides the core data structures and functionality for
//! storing and retrieving knowledge base tuples using redb.
//!
#[cfg(test)]
mod tests;
pub mod tuple;

pub use tuple::Ent;
pub use tuple::{Tuple, TupleID};

use anyhow::{Context, Result};
use redb::{Database, MultimapTableDefinition, ReadableDatabase, ReadableTable, TableDefinition};
use std::collections::HashSet;
use std::path::PathBuf;

const TUPLES_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("tuples");
const PREDICATE_TABLE: MultimapTableDefinition<&str, u64> =
    MultimapTableDefinition::new("tuples_by_predicate");
const SUBJECT_TABLE: MultimapTableDefinition<&str, u64> =
    MultimapTableDefinition::new("tuples_by_subject");
const OBJECT_TABLE: MultimapTableDefinition<&str, u64> =
    MultimapTableDefinition::new("tuples_by_object");

/// A simple wrapper around redb's database to manage tuples.
pub struct KB {
    db: Database,
}

impl KB {
    /// Creates a new KnowledgeBase instance.
    pub fn new(path: PathBuf) -> Result<Self> {
        let db = Database::create(&path)
            .with_context(|| format!("opening kb database at {}", path.display()))?;

        // Initialize the table by opening a write transaction and committing it
        let write_txn = db
            .begin_write()
            .with_context(|| format!("starting transaction on db {}", path.display()))?;
        {
            let _table = write_txn.open_table(TUPLES_TABLE)?;
            let _pred_table = write_txn.open_multimap_table(PREDICATE_TABLE)?;
            let _sub_table = write_txn.open_multimap_table(SUBJECT_TABLE)?;
            let _obj_table = write_txn.open_multimap_table(OBJECT_TABLE)?;
        }
        write_txn
            .commit()
            .with_context(|| format!("commiting transaction on db {}", path.display()))?;

        Ok(KB { db })
    }

    /// Inserts a tuple into the knowledge base.
    pub fn store_tuple(&self, tuple: &Tuple) -> Result<()> {
        self.store_tuples([tuple])
    }

    /// Inserts multiple tuples into the knowledge base in a single transaction.
    pub fn store_tuples<'a>(&self, tuples: impl IntoIterator<Item = &'a Tuple>) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(TUPLES_TABLE)?;
            let mut pred_table = write_txn.open_multimap_table(PREDICATE_TABLE)?;
            let mut sub_table = write_txn.open_multimap_table(SUBJECT_TABLE)?;
            let mut obj_table = write_txn.open_multimap_table(OBJECT_TABLE)?;

            for tuple in tuples {
                let id = tuple.id();
                let value = serde_json::to_vec(tuple)?;

                table.insert(id, value.as_slice())?;
                pred_table.insert(format!("{}", tuple.predicate).as_str(), &id)?;
                sub_table.insert(format!("{}", tuple.subject).as_str(), &id)?;
                obj_table.insert(format!("{}", tuple.object).as_str(), &id)?;
            }
        }
        write_txn.commit()?;
        Ok(())
    }

    /// Retrieves a tuple from the knowledge base by its ID.
    pub fn retrieve_by_id(&self, id: TupleID) -> Result<Option<Tuple>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(TUPLES_TABLE)?;
        if let Some(row) = table.get(&id)? {
            let tuple: Tuple = serde_json::from_slice(row.value())?;
            return Ok(Some(tuple));
        }

        Ok(None)
    }

    /// Retrieves a tuple from the knowledge base by its key (subject, predicate, object).
    pub fn retrieve_tuple(
        &self,
        subject: impl Into<Ent>,
        predicate: impl Into<Ent>,
        object: impl Into<Ent>,
    ) -> Result<Tuple> {
        let tuple = Tuple::new(subject.into(), predicate.into(), object.into(), 0.5);
        let tuple = match self.retrieve_by_id(tuple.id())? {
            Some(tuple) => tuple,
            None => tuple,
        };
        Ok(tuple)
    }

    /// Streams all tuples from the knowledge base, executing a closure for each.
    pub fn for_each_tuple<F>(&self, mut f: F) -> Result<usize>
    where
        F: FnMut(Tuple),
    {
        let mut count = 0;
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(TUPLES_TABLE)?;

        for row in table.iter()? {
            let (_key, val) = row?;
            let tuple: Tuple = serde_json::from_slice(val.value())?;
            f(tuple);
            count += 1;
        }

        Ok(count)
    }

    /// Retrieves all tuples with a specific subject.
    pub fn retrieve_by_subject(&self, subject: impl Into<Ent>) -> Result<HashSet<TupleID>> {
        self.retrieve_by_index(SUBJECT_TABLE, subject.into())
    }

    /// Retrieves all tuples with a specific predicate.
    pub fn retrieve_by_predicate(&self, predicate: impl Into<Ent>) -> Result<HashSet<TupleID>> {
        self.retrieve_by_index(PREDICATE_TABLE, predicate.into())
    }

    /// Retrieves all tuples with a specific object.
    pub fn retrieve_by_object(&self, object: impl Into<Ent>) -> Result<HashSet<TupleID>> {
        self.retrieve_by_index(OBJECT_TABLE, object.into())
    }

    /// Helper to retrieve Tuples by an index key (subject/predicate/object).
    fn retrieve_by_index(
        &self,
        table_def: MultimapTableDefinition<&str, u64>,
        entity: Ent,
    ) -> Result<HashSet<TupleID>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_multimap_table(table_def)?;
        let mut results = HashSet::new();

        for row in table.get(format!("{}", entity).as_str())? {
            let tuple: TupleID = row?.value();
            results.insert(tuple);
        }

        Ok(results)
    }

    /// Retrieves multiple tuples from the knowledge base by their IDs.
    pub fn retrieve_multiple_by_ids<'a>(
        &self,
        ids: impl IntoIterator<Item = &'a TupleID>,
    ) -> Result<Vec<Tuple>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(TUPLES_TABLE)?;
        let mut results = Vec::new();

        for id in ids {
            if let Some(row) = table.get(*id)? {
                let tuple: Tuple = serde_json::from_slice(row.value())?;
                results.push(tuple);
            }
        }
        Ok(results)
    }
}
