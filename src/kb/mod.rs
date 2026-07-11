//! Knowledge Base module for Logos, using sled as an embedded database.
//!
//! This module provides the core data structures and functionality for
//! storing and retrieving knowledge base tuples using sled.

pub mod tuple;
#[cfg(test)]
mod tests;

pub use tuple::Tuple;

use sled::{Db};
use std::error::Error;
use std::result;

/// A simple wrapper around sled's database to manage tuples.
pub struct KB {
    db: Db,
}

impl KB {
    /// Creates a new KnowledgeBase instance.
    pub fn new(path: impl AsRef<std::path::Path>) -> result::Result<Self, Box<dyn Error>> {
        let db = sled::open(path)?;
        Ok(KB { db })
    }

    /// Inserts a tuple into the knowledge base.
    pub fn store_tuple(&self, tuple: &Tuple) -> result::Result<(), Box<dyn Error>> {
        let key = format!("{}::{}::{}", tuple.subject, tuple.predicate, tuple.object);
        let value = serde_json::to_vec(tuple)?;
        self.db.insert(key.as_bytes(), value)?;
        Ok(())
    }

    /// Retrieves a tuple from the knowledge base by its key.
    pub fn retrieve_tuple(
        &self,
        subject: &str,
        predicate: &str,
        object: &str,
    ) -> result::Result<Option<Tuple>, Box<dyn Error>> {
        let key = format!("{}::{}::{}", subject, predicate, object);
        if let Some(value) = self.db.get(key.as_bytes())? {
            let tuple: Tuple = serde_json::from_slice(&value)?;
            Ok(Some(tuple))
        } else {
            Ok(None)
        }
    }

    /// Iterates over all tuples in the knowledge base.
    pub fn iter(&self) -> impl Iterator<Item = result::Result<Tuple, Box<dyn Error>>> {
        self.db.iter().map(|result| {
            let (_, value) = result.map_err(|e| -> Box<dyn Error> { e.into() })?;
            let tuple: Tuple = serde_json::from_slice(&value).map_err(|e| -> Box<dyn Error> { e.into() })?;
            Ok(tuple)
        })
    }

    /// Shuts down the database.
    pub fn shutdown(self) -> result::Result<(), Box<dyn Error>> {
        self.db.flush()?;
        Ok(())
    }
}
