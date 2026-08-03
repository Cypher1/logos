//! Definition of Queries in the knowledge base system.
//! Queries are structured requests for tuples matching specific templates.

use crate::kb::tuple::TupleTemplate;
use serde::{Deserialize, Serialize};

/// The category of question being asked, used to determine which inference engine to trigger.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum QueryType {
    /// Simple retrieval: find facts matching a specific pattern (e.g., (?x, knows, "Bob")).
    Lookup,
    /// Logic-based query: use rules to derive new information from existing facts based on premises.
    Inference,
    /// Quantitative or recursive exploration of the KB for high-level summaries.
    Aggregation,
}

/// A structured Query representing a request for knowledge with a specific structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Query {
    /// The TemplateTuple (pattern) this query is searching for.
    /// Example: "Who knows Bob?" becomes (?x, knows, "Bob")
    pub target_template: TupleTemplate,
    /// Categorization of the query to select the appropriate solver.
    pub q_type: QueryType,
    /// Confidence/Priority weight of this query (range 0.0 - 1.0).
    pub confidence: f32,
}

impl Query {
    /// Creates a new structured query for information retrieval.
    pub fn new(target_template: TupleTemplate, q_type: QueryType, confidence: f32) -> Self {
        Query {
            target_template,
            q_type,
            confidence,
        }
    }

    /// Shortcut for simple structural lookups.
    pub fn lookup(template: TupleTemplate) -> Self {
        Self::new(template, QueryType::Lookup, 1.0)
    }
}

impl std::fmt::Display for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Query ({:?}): {} [Conf: {:.2}]",
            self.q_type, self.target_template, self.confidence
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kb::tuple::{Slot, TupleTemplate};

    #[test]
    fn test_query_creation() {
        // Representing "Who knows Bob?" as a template: (?x, knows, "Bob")
        let t = TupleTemplate::new(
            Slot::Placeholder("x".into()),
            Slot::Constant("knows".into()),
            Slot::Constant("Bob".into()),
            1.0,
        );
        let q = Query::new(t, QueryType::Lookup, 0.9);
        assert_eq!(q.target_template.object, Slot::Constant("Bob".into()));
    }

    #[test]
    fn test_lookup_shortcut() {
        let t = TupleTemplate::new(
            Slot::Placeholder("x".into()),
            Slot::Constant("is_friend_of".into()),
            Slot::Placeholder("y".into()),
            1.0,
        );
        let q = Query::lookup(t);
        assert_eq!(q.q_type, QueryType::Lookup);
    }
}
