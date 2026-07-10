//! Definition of a canonical tuple for the knowledge base.

use serde::{Deserialize, Serialize};

/// A canonical tuple representation in the knowledge base.
///
/// This structure defines the basic elements of a fact or statement:
/// subject -> predicate -> object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tuple {
    /// The subject of the tuple.
    pub subject: String,

    /// The predicate of the tuple.
    pub predicate: String,

    /// The object of the tuple.
    pub object: String,
}

impl Tuple {
    /// Creates a new tuple.
    pub fn new(subject: impl Into<String>, predicate: impl Into<String>, object: impl Into<String>) -> Self {
        Tuple {
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
        }
    }

    /// Converts the tuple to a human-readable string representation.
    ///
    /// The format is: "subject predicate object"
    pub fn to_string(&self) -> String {
        format!("{} {} {}", self.subject, self.predicate, self.object)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tuple() {
        let tuple = Tuple::new("Alice", "knows", "Bob");
        assert_eq!(tuple.subject, "Alice");
        assert_eq!(tuple.predicate, "knows");
        assert_eq!(tuple.object, "Bob");
    }
}
