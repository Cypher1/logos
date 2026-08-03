//! Tests for the Knowledge Base inference engine.

use super::*;
use crate::kb::tuple::{Ent, Slot, Tuple};

#[test]
fn test_unification() {
    let facts = vec![
        Tuple::new(Ent::Str("Alice".into()), Ent::Str("knows".into()), Ent::Str("Bob".into()), 1.0),
        Tuple::new(Ent::Str("Bob".into()), Ent::Str("likes".into()), Ent::Str("Charlie".into()), 1.0),
    ];
    let rules = vec![];

    let engine = InferenceEngine::new(facts, rules);

    // Query: ?x knows "Bob"
    let query_template = TupleTemplate::new(
        Slot::Placeholder("x".into()),
        Slot::Constant("knows".into()),
        Slot::Constant("Bob".into()),
        1.0,
    );

    let result = engine.unify(&query_template, &facts[0]);
    assert!(result.is_some());
    let bindings = result.unwrap();
    assert_eq!(bindings.get("x").unwrap(), &Ent::Str("Alice".into()));
}

#[test]
fn test_rule_inference() {
    // Rule: ?x knows ?y => ?y likes ?x (Universal)
    let p1 = TupleTemplate::new(
        Slot::Placeholder("x".into()),
        Slot::Constant("knows".into()),
        Slot::Placeholder("y".into()),
        1.0,
    );

    let conclusion = TupleTemplate::new(
        Slot::Placeholder("y".into()),
        Slot::Constant("likes".into()),
        Slot::Placeholder("x".into()),
        1.0,
    );

    let rule = Rule::new(vec![p1], conclusion, Quantifier::Universal, 0.95);
    
    let facts = vec![
        Tuple::new(Ent::Str("Alice".into()), Ent::Str("knows".into()), Ent::Str("Bob".into()), 1.0),
    ];

    let engine = InferenceEngine::new(facts, vec![rule]);

    // Query: ?y likes "Alice" (Should find Bob knows Alice -> Bob likes Alice)
    // Wait, the rule is: ?x knows ?y => ?y likes ?x. 
    // If we have Alice knows Bob, then conclusion is Bob likes Alice.
    let query_template = TupleTemplate::new(
        Slot::Placeholder("y".into()),
        Slot::Constant("likes".into()),
        Slot::Constant("Alice".into()),
        1.0,
    );

    let result = engine.query(&Query::new(query_template, QueryType::Lookup));
    assert!(result.is_some());
}

#[test]
fn test_multiple_unification() {
    let facts = vec![
        Tuple::new(Ent::Str("Alice".into()), Ent::Str("knows".into()), Ent::Str("Bob".into()), 1.0),
        Tuple::new(Ent::Str("Charlie".into()), Ent::Str("knows".into()), Ent::Str("Bob".into()), 1.0),
    ];
    let rules = vec![];

    let engine = InferenceEngine::new(facts, rules);

    // Query: ?x knows "Bob"
    let query_template = TupleTemplate::new(
        Slot::Placeholder("x".into()),
        Slot::Constant("knows".into()),
        Slot::Constant("Bob".into()),
        1.0,
    );

    let results = engine.query(&Query::new(query_template, QueryType::Lookup));
    assert_eq!(results.len(), 2);
}
