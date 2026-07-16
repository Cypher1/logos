//! Definition of a canonical tuple for the knowledge base.

use serde::{Deserialize, Serialize};

/// An entity in the storage system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash)]
pub enum Ent {
    /// A concept/idea.
    Entity(u64),
    /// A statement / condition.
    Tuple(u64),
    /// A raw string (for names etc.).
    Str(String),
    /// A raw int 64(for counting etc.).
    I64(i64),
}
use Ent::*;

impl From<&str> for Ent {
    fn from(s: &str) -> Ent {
        Str(s.into())
    }
}

impl std::fmt::Display for Ent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Entity(e) => write!(f, "Entity{e}"),
            Tuple(t) => write!(f, "Tuple{t}"),
            Str(s) => write!(f, "'{s}'"),
            I64(i) => write!(f, "{i}"),
        }
    }
}

/// A canonical tuple representation in the knowledge base.
///
/// This structure defines the basic elements of a fact or statement:
/// subject -> predicate -> object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tuple {
    /// The subject of the tuple.
    pub subject: Ent,

    /// The predicate of the tuple.
    pub predicate: Ent,

    /// The object of the tuple.
    pub object: Ent,

    /// The confidence of the tuple (false: 0, true: 1)
    pub confidence: f32,
    // TODO: Add an optional time.
    // TODO: Add an optional world.
}

impl Tuple {
    /// Creates a new tuple.
    pub fn new(
        subject: impl Into<Ent>,
        predicate: impl Into<Ent>,
        object: impl Into<Ent>,
        confidence: f32,
    ) -> Self {
        Tuple {
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
            confidence,
        }
    }

    /// Calculates an ID from the hash.
    pub fn id(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut s = DefaultHasher::new();
        self.subject.hash(&mut s);
        self.predicate.hash(&mut s);
        self.object.hash(&mut s);
        s.finish()
    }
}

impl std::fmt::Display for Tuple {
    /// Converts the tuple to a human-readable string representation.
    ///
    /// The format is: "subject predicate object (confidence)"
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} ({:.2})",
            self.subject, self.predicate, self.object, self.confidence
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tuple() {
        let tuple = Tuple::new("Alice", "knows", "Bob", 0.9);
        assert_eq!(tuple.subject, "Alice".into());
        assert_eq!(tuple.predicate, "knows".into());
        assert_eq!(tuple.object, "Bob".into());
        assert!(tuple.confidence > 0.8999);
        assert!(tuple.confidence < 0.9001);
        assert_eq!(format!("{}", tuple), "'Alice' 'knows' 'Bob' (0.90)");
    }
}
