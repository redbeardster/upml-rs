//! State machine validation and correctness checking

use crate::state_machine::StateMachine;
use crate::Result;
use std::collections::{HashSet, HashMap};

#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub issue_type: IssueType,
    pub message: String,
    pub location: Option<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IssueType {
    UnreachableState,
    DeadState,
    MissingInitialState,
    MultipleInitialStates,
    UndefinedEvent,
    NonDeterministicTransition,
    OrphanedState,
    EmptyGuard,
    EmptyEffect,
    CyclicDependency,
}

impl std::fmt::Display for IssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueType::UnreachableState => write!(f, "Unreachable State"),
            IssueType::DeadState => write!(f, "Dead State"),
            IssueType::MissingInitialState => write!(f, "Missing Initial State"),
            IssueType::MultipleInitialStates => write!(f, "Multiple Initial States"),
            IssueType::UndefinedEvent => write!(f, "Undefined Event"),
            IssueType::NonDeterministicTransition => write!(f, "Non-Deterministic Transition"),
            IssueType::OrphanedState => write!(f, "Orphaned State"),
            IssueType::EmptyGuard => write!(f, "Empty Guard"),
            IssueType::EmptyEffect => write!(f, "Empty Effect"),
            IssueType::CyclicDependency => write!(f, "Cyclic Dependency"),
        }
    }
}

#[derive(Debug)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
    pub summary: ValidationSummary,
}

#[derive(Debug)]
pub struct ValidationSummary {
    pub total_states: usize,
    pub total_transitions: usize,
    pub total_events: usize,
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
}

pub struct StateMachineValidator;

impl StateMachineValidator {
    pub fn new() -> Self {
        Self
    }

    /// Validate a state machine and return a comprehensive report
    pub fn validate(&self, state_machine: &StateMachine) -> Result<ValidationReport> {
        let mut issues = Vec::new();

        // Run all validation checks
        issues.extend(self.check_initial_states(state_machine)?);
        issues.extend(self.check_reachability(state_machine)?);
        issues.extend(self.check_determinism(state_machine)?);
        issues.extend(self.check_undefined_events(state_machine)?);
        issues.extend(self.check_orphaned_states(state_machine)?);
        issues.extend(self.check_dead_states(state_machine)?);

        let summary = self.create_summary(state_machine, &issues);

        Ok(ValidationReport { issues, summary })
    }

    /// Check for proper initial state configuration
    fn check_initial_states(&self, state_machine: &StateMachine) -> Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();
        let initial_states = state_machine.initial_states();

        if initial_states.is_empty() {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                issue_type: IssueType::MissingInitialState,
                message: "State machine has no initial state".to_string(),
                location: Some(state_machine.id.clone()),
                suggestions: vec![
                    "Add a transition from [*] to define an initial state".to_string(),
                    "Mark a state as initial using the initial() method".to_string(),
                ],
            });
        } else if initial_states.len() > 1 {
            issues.push(ValidationIssue {
                severity: Severity::Warning,
                issue_type: IssueType::MultipleInitialStates,
                message: format!("Multiple initial states found: {:?}", initial_states),
                location: Some(state_machine.id.clone()),
                suggestions: vec![
                    "Consider using a single initial state".to_string(),
                    "Use composite states if multiple entry points are needed".to_string(),
                ],
            });
        }

        Ok(issues)
    }

    /// Check for unreachable states
    fn check_reachability(&self, state_machine: &StateMachine) -> Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();
        let reachable_states = self.find_reachable_states(state_machine);
        let all_states: HashSet<String> = state_machine.states(false).into_iter().collect();

        for state_id in all_states.difference(&reachable_states) {
            issues.push(ValidationIssue {
                severity: Severity::Warning,
                issue_type: IssueType::UnreachableState,
                message: format!("State '{}' is unreachable", state_id),
                location: Some(state_id.clone()),
                suggestions: vec![
                    "Add a transition to this state".to_string(),
                    "Remove the state if it's not needed".to_string(),
                    "Check if this state should be an initial state".to_string(),
                ],
            });
        }

        Ok(issues)
    }

    /// Check for non-deterministic transitions
    fn check_determinism(&self, state_machine: &StateMachine) -> Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();
        let mut state_event_map: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();

        // Collect all transitions grouped by (state, event)
        for region in state_machine.regions.values() {
            for state in region.borrow().substates.values() {
                let state_borrow = state.borrow();
                for transition in state_borrow.transitions.values() {
                    let state_events = state_event_map
                        .entry(transition.from_state.clone())
                        .or_insert_with(HashMap::new);
                    let transitions = state_events
                        .entry(transition.event.clone())
                        .or_insert_with(Vec::new);
                    transitions.push(transition.to_state.clone());
                }
            }
        }

        // Check for non-determinism
        for (state_id, events) in state_event_map {
            for (event_id, target_states) in events {
                if target_states.len() > 1 {
                    issues.push(ValidationIssue {
                        severity: Severity::Warning,
                        issue_type: IssueType::NonDeterministicTransition,
                        message: format!(
                            "Non-deterministic transition from '{}' on event '{}' to states: {:?}",
                            state_id, event_id, target_states
                        ),
                        location: Some(format!("{}:{}", state_id, event_id)),
                        suggestions: vec![
                            "Add guards to make transitions deterministic".to_string(),
                            "Use different events for different target states".to_string(),
                            "Consider if this non-determinism is intentional".to_string(),
                        ],
                    });
                }
            }
        }

        Ok(issues)
    }

    /// Check for undefined events
    fn check_undefined_events(&self, state_machine: &StateMachine) -> Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();
        let events = state_machine.events();
        
        // Check for events that might be typos or undefined
        let mut event_usage: HashMap<String, usize> = HashMap::new();
        for event in &events {
            *event_usage.entry(event.clone()).or_insert(0) += 1;
        }

        for (event, count) in event_usage {
            if count == 1 && !event.starts_with("NullEvent") {
                issues.push(ValidationIssue {
                    severity: Severity::Info,
                    issue_type: IssueType::UndefinedEvent,
                    message: format!("Event '{}' is used only once - possible typo?", event),
                    location: Some(event.clone()),
                    suggestions: vec![
                        "Check if this event name is correct".to_string(),
                        "Consider if this event should be used elsewhere".to_string(),
                    ],
                });
            }
        }

        Ok(issues)
    }

    /// Check for orphaned states (states with no incoming or outgoing transitions)
    fn check_orphaned_states(&self, state_machine: &StateMachine) -> Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();
        let mut states_with_incoming: HashSet<String> = HashSet::new();
        let mut states_with_outgoing: HashSet<String> = HashSet::new();

        // Collect states with transitions
        for region in state_machine.regions.values() {
            for state in region.borrow().substates.values() {
                let state_borrow = state.borrow();
                if !state_borrow.transitions.is_empty() {
                    states_with_outgoing.insert(state_borrow.id.clone());
                }
                
                for transition in state_borrow.transitions.values() {
                    states_with_incoming.insert(transition.to_state.clone());
                }
            }
        }

        let all_states: HashSet<String> = state_machine.states(false).into_iter().collect();
        let initial_states: HashSet<String> = state_machine.initial_states().into_iter().collect();

        for state_id in &all_states {
            let has_incoming = states_with_incoming.contains(state_id) || initial_states.contains(state_id);
            let has_outgoing = states_with_outgoing.contains(state_id);

            if !has_incoming && !has_outgoing {
                issues.push(ValidationIssue {
                    severity: Severity::Warning,
                    issue_type: IssueType::OrphanedState,
                    message: format!("State '{}' has no incoming or outgoing transitions", state_id),
                    location: Some(state_id.clone()),
                    suggestions: vec![
                        "Add transitions to/from this state".to_string(),
                        "Remove the state if it's not needed".to_string(),
                        "Consider making this an initial or final state".to_string(),
                    ],
                });
            }
        }

        Ok(issues)
    }

    /// Check for dead states (states with no outgoing transitions that aren't final)
    fn check_dead_states(&self, state_machine: &StateMachine) -> Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();
        let final_states: HashSet<String> = state_machine.final_states().into_iter().collect();

        for region in state_machine.regions.values() {
            for state in region.borrow().substates.values() {
                let state_borrow = state.borrow();
                if state_borrow.transitions.is_empty() && !final_states.contains(&state_borrow.id) {
                    issues.push(ValidationIssue {
                        severity: Severity::Info,
                        issue_type: IssueType::DeadState,
                        message: format!("State '{}' has no outgoing transitions and is not marked as final", state_borrow.id),
                        location: Some(state_borrow.id.clone()),
                        suggestions: vec![
                            "Add outgoing transitions from this state".to_string(),
                            "Mark this state as final if appropriate".to_string(),
                            "Add a transition to [*] to make it a final state".to_string(),
                        ],
                    });
                }
            }
        }

        Ok(issues)
    }

    /// Find all reachable states using DFS
    fn find_reachable_states(&self, state_machine: &StateMachine) -> HashSet<String> {
        let mut reachable = HashSet::new();
        let mut stack = Vec::new();
        
        // Start from initial states
        for initial_state in state_machine.initial_states() {
            stack.push(initial_state);
        }

        while let Some(current_state) = stack.pop() {
            if reachable.insert(current_state.clone()) {
                // Find all states reachable from current_state
                for region in state_machine.regions.values() {
                    if let Some(state) = region.borrow().substates.get(&current_state) {
                        let state_borrow = state.borrow();
                        for transition in state_borrow.transitions.values() {
                            if !reachable.contains(&transition.to_state) {
                                stack.push(transition.to_state.clone());
                            }
                        }
                    }
                }
            }
        }

        reachable
    }

    /// Create validation summary
    fn create_summary(&self, state_machine: &StateMachine, issues: &[ValidationIssue]) -> ValidationSummary {
        let errors = issues.iter().filter(|i| i.severity == Severity::Error).count();
        let warnings = issues.iter().filter(|i| i.severity == Severity::Warning).count();
        let infos = issues.iter().filter(|i| i.severity == Severity::Info).count();

        ValidationSummary {
            total_states: state_machine.states(false).len(),
            total_transitions: state_machine.transitions().len(),
            total_events: state_machine.events().len(),
            errors,
            warnings,
            infos,
        }
    }
}

impl Default for StateMachineValidator {
    fn default() -> Self {
        Self::new()
    }
}