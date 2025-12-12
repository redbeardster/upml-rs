//! State machine complexity metrics and analysis

use crate::state_machine::StateMachine;
use crate::Result;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct ComplexityMetrics {
    pub cyclomatic_complexity: usize,
    pub state_complexity: usize,
    pub transition_complexity: usize,
    pub depth_complexity: usize,
    pub fan_out_complexity: f64,
    pub fan_in_complexity: f64,
    pub coupling_factor: f64,
    pub cohesion_factor: f64,
}

#[derive(Debug, Clone)]
pub struct StateMetrics {
    pub id: String,
    pub incoming_transitions: usize,
    pub outgoing_transitions: usize,
    pub fan_out: usize,
    pub fan_in: usize,
    pub depth_level: usize,
    pub is_critical: bool,
}

#[derive(Debug, Clone)]
pub struct TransitionMetrics {
    pub from_state: String,
    pub to_state: String,
    pub event: String,
    pub has_guard: bool,
    pub has_effect: bool,
    pub guard_complexity: usize,
    pub effect_complexity: usize,
}

pub struct MetricsAnalyzer;

impl MetricsAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Calculate comprehensive complexity metrics for a state machine
    pub fn analyze_complexity(&self, state_machine: &StateMachine) -> Result<ComplexityMetrics> {
        let states = state_machine.states(false);
        let transitions = state_machine.transitions();
        let _events = state_machine.events();

        let cyclomatic_complexity = self.calculate_cyclomatic_complexity(state_machine);
        let state_complexity = states.len();
        let transition_complexity = transitions.len();
        let depth_complexity = state_machine.depth();
        let (fan_out_complexity, fan_in_complexity) = self.calculate_fan_metrics(state_machine);
        let coupling_factor = self.calculate_coupling_factor(state_machine);
        let cohesion_factor = self.calculate_cohesion_factor(state_machine);

        Ok(ComplexityMetrics {
            cyclomatic_complexity,
            state_complexity,
            transition_complexity,
            depth_complexity,
            fan_out_complexity,
            fan_in_complexity,
            coupling_factor,
            cohesion_factor,
        })
    }

    /// Calculate metrics for individual states
    pub fn analyze_states(&self, state_machine: &StateMachine) -> Result<Vec<StateMetrics>> {
        let mut state_metrics = Vec::new();
        let mut incoming_count: HashMap<String, usize> = HashMap::new();
        let mut outgoing_count: HashMap<String, usize> = HashMap::new();

        // Count incoming and outgoing transitions
        for region in state_machine.regions.values() {
            for state in region.borrow().substates.values() {
                let state_borrow = state.borrow();
                let outgoing = state_borrow.transitions.len();
                outgoing_count.insert(state_borrow.id.clone(), outgoing);

                for transition in state_borrow.transitions.values() {
                    *incoming_count.entry(transition.to_state.clone()).or_insert(0) += 1;
                }
            }
        }

        // Calculate metrics for each state
        for region in state_machine.regions.values() {
            for state in region.borrow().substates.values() {
                let state_borrow = state.borrow();
                let incoming = incoming_count.get(&state_borrow.id).unwrap_or(&0);
                let outgoing = outgoing_count.get(&state_borrow.id).unwrap_or(&0);

                let metrics = StateMetrics {
                    id: state_borrow.id.clone(),
                    incoming_transitions: *incoming,
                    outgoing_transitions: *outgoing,
                    fan_out: *outgoing,
                    fan_in: *incoming,
                    depth_level: self.calculate_state_depth(&state_borrow.id, state_machine),
                    is_critical: self.is_critical_state(&state_borrow.id, state_machine),
                };

                state_metrics.push(metrics);
            }
        }

        Ok(state_metrics)
    }

    /// Calculate metrics for individual transitions
    pub fn analyze_transitions(&self, state_machine: &StateMachine) -> Result<Vec<TransitionMetrics>> {
        let mut transition_metrics = Vec::new();

        for region in state_machine.regions.values() {
            for state in region.borrow().substates.values() {
                let state_borrow = state.borrow();
                for transition in state_borrow.transitions.values() {
                    let guard_complexity = transition.guard.len();
                    let effect_complexity = transition.effect.len();

                    let metrics = TransitionMetrics {
                        from_state: transition.from_state.clone(),
                        to_state: transition.to_state.clone(),
                        event: transition.event.clone(),
                        has_guard: !transition.guard.is_empty(),
                        has_effect: !transition.effect.is_empty(),
                        guard_complexity,
                        effect_complexity,
                    };

                    transition_metrics.push(metrics);
                }
            }
        }

        Ok(transition_metrics)
    }

    /// Calculate cyclomatic complexity (edges - nodes + 2)
    fn calculate_cyclomatic_complexity(&self, state_machine: &StateMachine) -> usize {
        let states = state_machine.states(false).len();
        let transitions = state_machine.transitions().len();
        
        // Cyclomatic complexity = E - N + 2P (where P is number of connected components)
        // For simplicity, assume P = 1 (single connected component)
        if transitions >= states {
            transitions - states + 2
        } else {
            1 // Minimum complexity
        }
    }

    /// Calculate fan-in and fan-out metrics
    fn calculate_fan_metrics(&self, state_machine: &StateMachine) -> (f64, f64) {
        let mut fan_out_sum = 0;
        let mut fan_in_count: HashMap<String, usize> = HashMap::new();
        let mut state_count = 0;

        for region in state_machine.regions.values() {
            for state in region.borrow().substates.values() {
                let state_borrow = state.borrow();
                state_count += 1;
                fan_out_sum += state_borrow.transitions.len();

                for transition in state_borrow.transitions.values() {
                    *fan_in_count.entry(transition.to_state.clone()).or_insert(0) += 1;
                }
            }
        }

        let avg_fan_out = if state_count > 0 {
            fan_out_sum as f64 / state_count as f64
        } else {
            0.0
        };

        let avg_fan_in = if state_count > 0 {
            fan_in_count.values().sum::<usize>() as f64 / state_count as f64
        } else {
            0.0
        };

        (avg_fan_out, avg_fan_in)
    }

    /// Calculate coupling factor (how interconnected states are)
    fn calculate_coupling_factor(&self, state_machine: &StateMachine) -> f64 {
        let states = state_machine.states(false);
        let transitions = state_machine.transitions();
        
        if states.len() <= 1 {
            return 0.0;
        }

        let max_possible_connections = states.len() * (states.len() - 1);
        if max_possible_connections == 0 {
            return 0.0;
        }

        transitions.len() as f64 / max_possible_connections as f64
    }

    /// Calculate cohesion factor (how well states work together)
    fn calculate_cohesion_factor(&self, state_machine: &StateMachine) -> f64 {
        let events = state_machine.events();
        let states = state_machine.states(false);
        
        if events.is_empty() || states.is_empty() {
            return 0.0;
        }

        // Simple cohesion metric: ratio of shared events to total events
        let mut event_usage: HashMap<String, usize> = HashMap::new();
        
        for region in state_machine.regions.values() {
            for state in region.borrow().substates.values() {
                let state_borrow = state.borrow();
                for transition in state_borrow.transitions.values() {
                    *event_usage.entry(transition.event.clone()).or_insert(0) += 1;
                }
            }
        }

        let shared_events = event_usage.values().filter(|&&count| count > 1).count();
        shared_events as f64 / events.len() as f64
    }

    /// Calculate the depth level of a state in the hierarchy
    fn calculate_state_depth(&self, _state_id: &str, _state_machine: &StateMachine) -> usize {
        // Simplified implementation - in a full version, we'd traverse the hierarchy
        1
    }

    /// Determine if a state is critical (high fan-in/fan-out or on critical path)
    fn is_critical_state(&self, state_id: &str, state_machine: &StateMachine) -> bool {
        let initial_states: HashSet<String> = state_machine.initial_states().into_iter().collect();
        let final_states: HashSet<String> = state_machine.final_states().into_iter().collect();
        
        // A state is critical if it's initial, final, or has high connectivity
        initial_states.contains(state_id) || final_states.contains(state_id)
    }
}

impl Default for MetricsAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}