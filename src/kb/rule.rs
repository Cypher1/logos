//! Definition of rules in the knowledge base system.
//! Rules define logical implications using templates and placeholders.

use crate::kb::tuple::TupleTemplate;
use nom::Parser;
use serde::{Deserialize, Serialize};

/// A logical rule mapping premises to a conclusion via variable binding across templates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Rule {
    /// Premises are template tuples with placeholders (e.g., ?x, ?y).
    pub premises: Vec<TupleTemplate>,
    /// The resulting tuple/fact inferred from the premises.
    pub conclusion: Vec<TupleTemplate>,
    /// Confidence score (0.0 - 1.0) of the inference result.
    pub confidence: f32,
}

impl Rule {
    /// Creates a new rule.
    /// Premises and conclusion should use `TupleTemplate::new` to define placeholders.
    pub fn new(premises: Vec<TupleTemplate>, conclusion: TupleTemplate, confidence: f32) -> Self {
        Rule {
            premises,
            conclusion: vec![conclusion],
            confidence,
        }
    }
}

impl std::fmt::Display for Rule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Rule(Premises: {:?}, Conclusion: {:?}, Confidence: {:.2})",
            self.premises, self.conclusion, self.confidence
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kb::tuple::{Slot, Tuple};

    #[test]
    fn test_template_rule() {
        // Rule: ?x knows ?y  =>  ?y is informed by ?x (Universal)
        let p1 = TupleTemplate::new(
            Slot::Placeholder("x".into()),
            Slot::Constant("knows".into()),
            Slot::Placeholder("y".into()),
            1.0,
        );

        let conc = TupleTemplate::new(
            Slot::Placeholder("y".into()),
            Slot::Constant("informed_by".into()),
            Slot::Placeholder("?x".into()),
            1.0,
        );

        let rule = Rule::new(vec![p1.clone()], conc, 0.95);
        assert_eq!(rule.confidence, 0.95);
    }
}
