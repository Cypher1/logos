//! Inference engine for the Logos Knowledge Base.
//! Handles variable binding, rule applications, and query resolution.

use crate::kb::query::Query;
use crate::kb::rule::Rule;
use crate::kb::tuple::{Ent, Slot, Tuple, TupleTemplate};
use std::collections::HashMap;

/// A mapping from placeholder names (e.g., "?x") to actual entities.
pub type Bindings = HashMap<String, Ent>;

#[derive(Debug)]
pub enum InferenceError {
    NoBindingsFound,
    CycleDetected,
}

pub struct InferenceEngine<'a> {
    /// The set of facts currently known by the system.
    pub facts: Vec<Tuple>,
    /// Pre-indexed rules for faster lookup.
    pub rules: Vec<Rule>,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> InferenceEngine<'a> {
    pub fn new(facts: Vec<Tuple>, rules: Vec<Rule>) -> Self {
        Self {
            facts,
            rules,
            _marker: std::marker::PhantomData,
        }
    }

    /// Attempts to unify a template tuple with a concrete fact or another template.
    /// Returns a set of bindings if successful, otherwise an empty map.
    pub fn unify(&self, template: &TupleTemplate, target: &impl Unifiable) -> Option<Bindings> {
        let mut bindings: HashMap<String, Ent> = HashMap::new();

        for (t_slot, target_slot) in [
            (&template.subject, &target.subject()),
            (&template.predicate, &target.predicate()),
            (&template.object, &target.object()),
        ]
        .iter()
        {
            match (t_slot, target_slot) {
                (Slot::Placeholder(name), ent) => {
                    if bindings.contains_key(name) {
                        if bindings.get(name) != Some(ent) {
                            return None;
                        }
                    } else {
                        // Since 'target' is Unifiable, 'ent' here is &Slot.
                        // We need to clone the inner value.
                        bindings.insert(name.clone(), ent.clone());
                    }
                }
                (Slot::Constant(e1), e2) => {
                    if &e1 != e2 {
                        return None;
                    }
                }
                _ => return None,
            }
        }

        Some(bindings)
    }

    /// Resolves a query by attempting to satisfy the target template via rules and facts.
    pub fn resolve(&self, query: &Query) -> Vec<(Tuple, f32)> {
        let mut results = Vec::new();

        // 1. Simple Lookup: check if any existing fact matches the target_template exactly (with variable binding).
        for fact in &self.facts {
            if let Some(_bindings) = self.unify(&query.target_template, fact) {
                results.push((fact.clone(), query.confidence));
            }
        }

        // 2. Rule-based Inference:
        // For every rule whose conclusion matches the structure of our target template...
        for rule in &self.rules {
            if let Some(conclusion_bindings) =
                self.unify(&query.target_template, &rule.conclusion[0])
            {
                // Check if premises can be satisfied by existing facts under these bindings.
                if let Some(_premise_bindings) = self.satisfy_premises(rule, conclusion_bindings) {
                    // If satisfied, the rule result is valid for this query template.
                    results.push((
                        rule.conclusion[0].clone(),
                        rule.confidence * query.confidence,
                    ));
                }
            }
        }

        results
    }

    /// Returns a set of bindings if all premises of a rule can be satisfied by known facts,
    /// given the initial bindings from a conclusion match.
    fn satisfy_premises(&self, rule: &Rule, mut current_bindings: Bindings) -> Option<Bindings> {
        // For simplicity, we assume a basic join operation across multiple premises.
        for premise in &rule.premises {
            let mut possible = false;
            for fact in &self.facts {
                if let Some(fact_bindings) = self.unify(premise, fact) {
                    // Try to merge the new bindings found from this fact into our current set.
                    // This is a simplified "join" logic.
                    let mut new_bindings = current_bindings.clone();
                    let mut conflict = false;

                    for (k, v) in fact_bindings {
                        match new_bindings.get(&k) {
                            Some(existing) if existing != &v => {
                                conflict = true;
                                break;
                            }
                            _ => {
                                new_bindings.insert(k, v);
                            }
                        }
                    }

                    if !conflict {
                        current_bindings = new_bindings;
                        possible = true;
                        break;
                    }
                }
            }

            if !possible {
                return None;
            }
        }

        Some(current_bindings)
    }
}
