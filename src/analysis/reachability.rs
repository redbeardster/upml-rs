//! Reachability analysis for state machines

use crate::state_machine::StateMachine;
use crate::Result;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
pub struct ReachabilityAnalysis {
    pub reachable_states: HashSet<String>,
    pub unreachable_states: HashSet<String>,
    pub reachability_graph: HashMap<String, HashSet<String>>,
    pub shortest_paths: HashMap<String, HashMap<String, Vec<String>>>,
    pub strongly_connected_components: Vec<HashSet<String>>,
}

#[derive(Debug, Clone)]
pub struct PathAnalysis {
    pub from_state: String,
    pub to_state: String,
    pub shortest_path: Option<Vec<String>>,
    pub all_paths: Vec<Vec<String>>,
    pub path_length: usize,
    pub is_reachable: bool,
}

pub struct ReachabilityAnalyzer;

impl ReachabilityAnalyzer {
    pub fn new() -> Self {
        Self
    }

    /// Perform comprehensive reachability analysis
    pub fn analyze(&self, state_machine: &StateMachine) -> Result<ReachabilityAnalysis> {
        let all_states: HashSet<String> = state_machine.states(false);
        let reachability_graph = self.build_reachability_graph(state_machine);
        let reachable_states = self.find_reachable_states(state_machine);
        let unreachable_states = all_states.difference(&reachable_states).cloned().collect();
        let shortest_paths = self.calculate_shortest_paths(state_machine, &reachability_graph);
        let strongly_connected_components = self.find_strongly_connected_components(&reachability_graph);

        Ok(ReachabilityAnalysis {
            reachable_states,
            unreachable_states,
            reachability_graph,
            shortest_paths,
            strongly_connected_components,
        })
    }

    /// Find path between two specific states
    pub fn find_path(&self, state_machine: &StateMachine, from: &str, to: &str) -> Result<PathAnalysis> {
        let reachability_graph = self.build_reachability_graph(state_machine);
        let shortest_path = self.find_shortest_path(&reachability_graph, from, to);
        let all_paths = self.find_all_paths(&reachability_graph, from, to, 10); // Limit to 10 paths
        let is_reachable = shortest_path.is_some();
        let path_length = shortest_path.as_ref().map(|p| p.len()).unwrap_or(0);

        Ok(PathAnalysis {
            from_state: from.to_string(),
            to_state: to.to_string(),
            shortest_path,
            all_paths,
            path_length,
            is_reachable,
        })
    }

    /// Build a graph of state-to-state reachability
    fn build_reachability_graph(&self, state_machine: &StateMachine) -> HashMap<String, HashSet<String>> {
        let mut graph = HashMap::new();

        for region in state_machine.regions.values() {
            for state in region.borrow().substates.values() {
                let state_borrow = state.borrow();
                let mut reachable = HashSet::new();

                for transition in state_borrow.transitions.values() {
                    reachable.insert(transition.to_state.clone());
                }

                graph.insert(state_borrow.id.clone(), reachable);
            }
        }

        graph
    }

    /// Find all reachable states from initial states using BFS
    fn find_reachable_states(&self, state_machine: &StateMachine) -> HashSet<String> {
        let mut reachable = HashSet::new();
        let mut queue = VecDeque::new();
        let reachability_graph = self.build_reachability_graph(state_machine);

        // Start from initial states
        for initial_state in state_machine.initial_states() {
            queue.push_back(initial_state.clone());
            reachable.insert(initial_state);
        }

        while let Some(current_state) = queue.pop_front() {
            if let Some(neighbors) = reachability_graph.get(&current_state) {
                for neighbor in neighbors {
                    if reachable.insert(neighbor.clone()) {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }

        reachable
    }

    /// Calculate shortest paths between all pairs of states using Floyd-Warshall
    fn calculate_shortest_paths(
        &self,
        state_machine: &StateMachine,
        reachability_graph: &HashMap<String, HashSet<String>>,
    ) -> HashMap<String, HashMap<String, Vec<String>>> {
        let states: Vec<String> = state_machine.states(false).into_iter().collect();
        let mut distances: HashMap<String, HashMap<String, Option<usize>>> = HashMap::new();
        let mut next_state: HashMap<String, HashMap<String, Option<String>>> = HashMap::new();

        // Initialize distances
        for state in &states {
            let mut state_distances = HashMap::new();
            let mut state_next = HashMap::new();

            for other_state in &states {
                if state == other_state {
                    state_distances.insert(other_state.clone(), Some(0));
                    state_next.insert(other_state.clone(), None);
                } else if reachability_graph
                    .get(state)
                    .map(|neighbors| neighbors.contains(other_state))
                    .unwrap_or(false)
                {
                    state_distances.insert(other_state.clone(), Some(1));
                    state_next.insert(other_state.clone(), Some(other_state.clone()));
                } else {
                    state_distances.insert(other_state.clone(), None);
                    state_next.insert(other_state.clone(), None);
                }
            }

            distances.insert(state.clone(), state_distances);
            next_state.insert(state.clone(), state_next);
        }

        // Floyd-Warshall algorithm
        for k in &states {
            for i in &states {
                for j in &states {
                    if let (Some(dist_ik), Some(dist_kj)) = (
                        distances[i][k],
                        distances[k][j],
                    ) {
                        let new_dist = dist_ik + dist_kj;
                        if distances[i][j].map(|d| new_dist < d).unwrap_or(true) {
                            distances.get_mut(i).unwrap().insert(j.clone(), Some(new_dist));
                            let next_k = next_state[i][k].clone();
                            next_state.get_mut(i).unwrap().insert(j.clone(), next_k);
                        }
                    }
                }
            }
        }

        // Reconstruct paths
        let mut paths = HashMap::new();
        for from_state in &states {
            let mut from_paths = HashMap::new();
            for to_state in &states {
                if let Some(_) = distances[from_state][to_state] {
                    let path = self.reconstruct_path(&next_state, from_state, to_state);
                    from_paths.insert(to_state.clone(), path);
                }
            }
            paths.insert(from_state.clone(), from_paths);
        }

        paths
    }

    /// Reconstruct path from next_state matrix
    fn reconstruct_path(
        &self,
        next_state: &HashMap<String, HashMap<String, Option<String>>>,
        from: &str,
        to: &str,
    ) -> Vec<String> {
        if next_state[from][to].is_none() {
            return vec![];
        }

        let mut path = vec![from.to_string()];
        let mut current = from;

        while current != to {
            if let Some(next) = &next_state[current][to] {
                path.push(next.clone());
                current = next;
            } else {
                break;
            }
        }

        path
    }

    /// Find shortest path between two states using BFS
    fn find_shortest_path(
        &self,
        reachability_graph: &HashMap<String, HashSet<String>>,
        from: &str,
        to: &str,
    ) -> Option<Vec<String>> {
        if from == to {
            return Some(vec![from.to_string()]);
        }

        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent: HashMap<String, String> = HashMap::new();

        queue.push_back(from.to_string());
        visited.insert(from.to_string());

        while let Some(current) = queue.pop_front() {
            if let Some(neighbors) = reachability_graph.get(&current) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        visited.insert(neighbor.clone());
                        parent.insert(neighbor.clone(), current.clone());
                        queue.push_back(neighbor.clone());

                        if neighbor == to {
                            // Reconstruct path
                            let mut path = vec![to.to_string()];
                            let mut current_node = to;

                            while let Some(p) = parent.get(current_node) {
                                path.push(p.clone());
                                current_node = p;
                            }

                            path.reverse();
                            return Some(path);
                        }
                    }
                }
            }
        }

        None
    }

    /// Find all paths between two states (limited by max_paths)
    fn find_all_paths(
        &self,
        reachability_graph: &HashMap<String, HashSet<String>>,
        from: &str,
        to: &str,
        max_paths: usize,
    ) -> Vec<Vec<String>> {
        let mut all_paths = Vec::new();
        let mut current_path = vec![from.to_string()];
        let mut visited = HashSet::new();

        self.dfs_all_paths(
            reachability_graph,
            from,
            to,
            &mut current_path,
            &mut visited,
            &mut all_paths,
            max_paths,
        );

        all_paths
    }

    /// DFS helper for finding all paths
    fn dfs_all_paths(
        &self,
        reachability_graph: &HashMap<String, HashSet<String>>,
        current: &str,
        target: &str,
        current_path: &mut Vec<String>,
        visited: &mut HashSet<String>,
        all_paths: &mut Vec<Vec<String>>,
        max_paths: usize,
    ) {
        if all_paths.len() >= max_paths {
            return;
        }

        if current == target {
            all_paths.push(current_path.clone());
            return;
        }

        visited.insert(current.to_string());

        if let Some(neighbors) = reachability_graph.get(current) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    current_path.push(neighbor.clone());
                    self.dfs_all_paths(
                        reachability_graph,
                        neighbor,
                        target,
                        current_path,
                        visited,
                        all_paths,
                        max_paths,
                    );
                    current_path.pop();
                }
            }
        }

        visited.remove(current);
    }

    /// Find strongly connected components using Tarjan's algorithm
    fn find_strongly_connected_components(
        &self,
        reachability_graph: &HashMap<String, HashSet<String>>,
    ) -> Vec<HashSet<String>> {
        let mut index = 0;
        let mut stack = Vec::new();
        let mut indices: HashMap<String, usize> = HashMap::new();
        let mut lowlinks: HashMap<String, usize> = HashMap::new();
        let mut on_stack: HashSet<String> = HashSet::new();
        let mut components = Vec::new();

        for node in reachability_graph.keys() {
            if !indices.contains_key(node) {
                self.tarjan_scc(
                    node,
                    reachability_graph,
                    &mut index,
                    &mut stack,
                    &mut indices,
                    &mut lowlinks,
                    &mut on_stack,
                    &mut components,
                );
            }
        }

        components
    }

    /// Tarjan's strongly connected components algorithm
    fn tarjan_scc(
        &self,
        node: &str,
        reachability_graph: &HashMap<String, HashSet<String>>,
        index: &mut usize,
        stack: &mut Vec<String>,
        indices: &mut HashMap<String, usize>,
        lowlinks: &mut HashMap<String, usize>,
        on_stack: &mut HashSet<String>,
        components: &mut Vec<HashSet<String>>,
    ) {
        indices.insert(node.to_string(), *index);
        lowlinks.insert(node.to_string(), *index);
        *index += 1;
        stack.push(node.to_string());
        on_stack.insert(node.to_string());

        if let Some(neighbors) = reachability_graph.get(node) {
            for neighbor in neighbors {
                if !indices.contains_key(neighbor) {
                    self.tarjan_scc(
                        neighbor,
                        reachability_graph,
                        index,
                        stack,
                        indices,
                        lowlinks,
                        on_stack,
                        components,
                    );
                    let neighbor_lowlink = lowlinks[neighbor];
                    let current_lowlink = lowlinks[node];
                    lowlinks.insert(node.to_string(), current_lowlink.min(neighbor_lowlink));
                } else if on_stack.contains(neighbor) {
                    let neighbor_index = indices[neighbor];
                    let current_lowlink = lowlinks[node];
                    lowlinks.insert(node.to_string(), current_lowlink.min(neighbor_index));
                }
            }
        }

        if lowlinks[node] == indices[node] {
            let mut component = HashSet::new();
            loop {
                let w = stack.pop().unwrap();
                on_stack.remove(&w);
                component.insert(w.clone());
                if w == node {
                    break;
                }
            }
            components.push(component);
        }
    }
}

impl Default for ReachabilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}