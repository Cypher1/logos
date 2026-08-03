//! Definition of canonical facts and templates in the knowledge base.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub type TupleID = u64;

/// An entity in the storage system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Ent {
    Entity(u64),
    Tuple(TupleID),
    Str(String),
    I64(i64),
}

impl From<&str> for Ent {
    fn from(s: &str) -> Ent {
        // TODO: Parse the str and any quantifiers.
        // Error out on quantifiers.
        Ent::Str(s.into())
    }
}

impl std::fmt::Display for Ent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ent::Entity(e) => write!(f, "Entity{}", e),
            Ent::Tuple(t) => write!(f, "Tuple{}", t),
            Ent::Str(s) => {
                if s.chars()
                    .any(|c| c.is_ascii_digit() || c == ' ' || c == '\'')
                {
                    write!(f, "'{}'", s)
                } else {
                    write!(f, "{}", s)
                }
            }
            Ent::I64(i) => write!(f, "{}", i),
        }
    }
}

/// Quantifiers for rule application to interpret placeholder scope.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Quantifier {
    /// Universal ($\forall$): Any substitution that satisfies the premise results in a conclusion.
    Universal,
    /// Existential ($\exists$): A conclusion is valid if there exists at least one substitution satisfying the premise.
    Existential,
}

impl std::fmt::Display for Quantifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Quantifier::Universal => write!(f, "forall"),
            Quantifier::Existential => write!(f, "exists"),
        }
    }
}

/// Represents a slot in a tuple which can be either a fixed Entity or
/// introduce or reuse a Placeholder.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Slot {
    /// A constant value from the knowledge base.
    Constant(Ent),
    /// An introduction of a variable (e.g., x, user). Internal string does NOT contain '?'.
    /// The quantier defines how variables in premises map to the conclusion.
    Introduction(Quantifier, String),
    /// A placeholder variable (e.g., x, user). Internal string does NOT contain '?'.
    Placeholder(String),
}

impl PartialOrd for Slot {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Slot {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Slot::Constant(e1), Slot::Constant(e2)) => e1.cmp(e2),
            (Slot::Introduction(q1, s1), Slot::Introduction(q2, s2)) => q1.cmp(q2).then(s1.cmp(s2)),
            (Slot::Placeholder(s1), Slot::Placeholder(s2)) => s1.cmp(s2),
            _ => Ordering::Equal, // Simplify comparison for mixed types (usually shouldn't happen in valid logic)
        }
    }
}

impl From<Ent> for Slot {
    fn from(ent: Ent) -> Self {
        Slot::Constant(ent)
    }
}

impl From<&str> for Slot {
    fn from(s: &str) -> Self {
        // TODO: Parse the str and any quantifiers.
        Slot::Constant(s.into())
    }
}

impl From<Slot> for Ent {
    fn from(slot: Slot) -> Self {
        match slot {
            Slot::Constant(e) => e,
            Slot::Introduction(_, s) | Slot::Placeholder(s) => Ent::Str(s),
        }
    }
}

impl std::fmt::Display for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Slot::Constant(e) => write!(f, "{}", e),
            // Prepend 'for all' or 'there exists' only during display.
            Slot::Introduction(q, p) => write!(f, "{} {}", q, p),
            // Prepend '?' only during display.
            Slot::Placeholder(p) => write!(f, "?{}", p),
        }
    }
}

/// A tuple representation that supports concrete facts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tuple {
    pub subject: Ent,
    pub predicate: Ent,
    pub object: Ent,
    pub confidence: f32,
}

impl Tuple {
    /// Creates a concrete fact where all slots are constants.
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

    /// Generates a unique ID based only a content hash.
    pub fn id(&self) -> TupleID {
        let mut s = DefaultHasher::new();
        self.subject.hash(&mut s);
        self.predicate.hash(&mut s);
        self.object.hash(&mut s);
        s.finish()
    }
}

impl std::fmt::Display for Tuple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {}: {:.2}",
            self.subject, self.predicate, self.object, self.confidence
        )
    }
}

impl Eq for Tuple {}

impl Ord for Tuple {
    fn cmp(&self, other: &Tuple) -> Ordering {
        let sub = self.subject.cmp(&other.subject);
        if sub != Ordering::Equal {
            return sub;
        }
        let pred = self.predicate.cmp(&other.predicate);
        if pred != Ordering::Equal {
            return pred;
        }
        let obj = self.object.cmp(&other.object);
        if obj != Ordering::Equal {
            return obj;
        }
        Ordering::Equal
    }
}

impl PartialOrd for Tuple {
    fn partial_cmp(&self, other: &Tuple) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A tuple representation that supports both concrete facts and abstract templates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TupleTemplate {
    pub subject: Slot,
    pub predicate: Slot,
    pub object: Slot,
    pub confidence: f32,
}

impl TupleTemplate {
    /// Creates a template tuple with placeholders (Variables).
    pub fn new(
        subject: impl Into<Slot>,
        predicate: impl Into<Slot>,
        object: impl Into<Slot>,
        confidence: f32,
    ) -> Self {
        TupleTemplate {
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
            confidence,
        }
    }

    /// Generates a unique ID based only a content hash.
    pub fn id(&self) -> TupleID {
        let mut s = DefaultHasher::new();
        self.subject.hash(&mut s);
        self.predicate.hash(&mut s);
        self.object.hash(&mut s);
        s.finish()
    }
}

impl std::fmt::Display for TupleTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {}: {:.2}",
            self.subject, self.predicate, self.object, self.confidence
        )
    }
}

impl Eq for TupleTemplate {}

impl Ord for TupleTemplate {
    fn cmp(&self, other: &TupleTemplate) -> Ordering {
        let sub = self.subject.cmp(&other.subject);
        if sub != Ordering::Equal {
            return sub;
        }
        let pred = self.predicate.cmp(&other.predicate);
        if pred != Ordering::Equal {
            return pred;
        }
        let obj = self.object.cmp(&other.object);
        if obj != Ordering::Equal {
            return obj;
        }
        Ordering::Equal
    }
}

impl PartialOrd for TupleTemplate {
    fn partial_cmp(&self, other: &TupleTemplate) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fact() {
        let t = Tuple::new("Alice", "knows", "Bob", 0.9);
        assert_eq!(t.subject, Ent::Str("Alice".into()));
    }

    #[test]
    fn test_template() {
        // Verify that we store the key WITHOUT the question mark internally
        let t = TupleTemplate::new(
            Slot::Placeholder("x".into()),
            Slot::Constant(Ent::Str("knows".into())),
            Slot::Placeholder("y".into()),
            1.0,
        );
        assert_eq!(t.subject, Slot::Placeholder("x".into()));
    }

    #[test]
    fn test_display() {
        // Verify that '?' is added back during display
        let t = TupleTemplate::new(
            Slot::Placeholder("x".into()),
            Slot::Constant(Ent::Str("knows".into())),
            Slot::Placeholder("y".into()),
            1.0,
        );
        let s = format!("{}", t);
        assert!(s.contains("?x"));
        assert!(s.contains("?y"));
    }

    #[test]
    fn test_id_uniqueness() {
        let t1 = TupleTemplate::new(
            Slot::Placeholder("x".into()),
            Slot::Constant(Ent::Str("knows".into())),
            Slot::Placeholder("y".into()),
            1.0,
        );
        let t2 = TupleTemplate::new(
            Slot::Placeholder("a".into()),
            Slot::Constant(Ent::Str("knows".into())),
            Slot::Placeholder("b".into()),
            1.0,
        );
        assert_eq!(t1.id(), t2.id());
    }
}
