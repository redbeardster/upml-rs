use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};
use std::cell::RefCell;
use serde::{Deserialize, Serialize};

/// Unique identifier type for states, events, transitions, etc.
pub type Id = String;

/// Set of names/identifiers
pub type Names = HashSet<Id>;

/// Source location information for debugging
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Location {
    pub line: usize,
    pub column: usize,
    pub file: String,
}

/// Event definition
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub id: Id,
}

/// Transition guard (boolean expression)
pub type Guard = Vec<String>;

/// Transition effect (list of statements)
pub type Effect = Vec<String>;

/// Activity arguments
pub type ActivityArgs = Vec<String>;

/// Transition between states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transition {
    pub id: Id,
    pub from_state: Id,
    pub to_state: Id,
    pub event: Id,
    pub guard: Guard,
    pub effect: Effect,
    pub location: Location,
}

/// Activity (entry, exit, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    pub id: Id,
    pub state: Id,
    pub activity_type: Id, // entry, exit, precondition, etc.
    pub args: ActivityArgs,
    pub location: Location,
}

/// State configuration options
pub type StateConfig = HashSet<String>;

/// Reference-counted state pointer
pub type StateRef = Rc<RefCell<State>>;
pub type WeakStateRef = Weak<RefCell<State>>;

/// Reference-counted region pointer  
pub type RegionRef = Rc<RefCell<Region>>;
pub type WeakRegionRef = Weak<RefCell<Region>>;

/// State in the state machine
#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pub id: Id,
    #[serde(skip)]
    pub super_state: Option<WeakStateRef>,
    #[serde(skip)]
    pub owned_by_region: Option<WeakRegionRef>,
    #[serde(skip)]
    pub regions: HashMap<Id, RegionRef>,
    pub transitions: HashMap<Id, Transition>,
    pub activities: Vec<Activity>,
    pub is_initial: bool,
    pub is_final: bool,
    pub config: StateConfig,
    pub location: Location,
    
    // Cached path to root (excluding this state)
    #[serde(skip)]
    #[allow(dead_code)]
    path_to_root: RefCell<Option<Vec<StateRef>>>,
}

impl State {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            super_state: None,
            owned_by_region: None,
            regions: HashMap::new(),
            transitions: HashMap::new(),
            activities: Vec::new(),
            is_initial: false,
            is_final: false,
            config: HashSet::new(),
            location: Location::default(),
            path_to_root: RefCell::new(None),
        }
    }

    /// Get all events from this state and its subregions
    pub fn events(&self) -> Names {
        let mut events = Names::new();
        
        // Events from regions
        for region in self.regions.values() {
            let region_events = region.borrow().events();
            events.extend(region_events);
        }
        
        // Events from transitions
        for transition in self.transitions.values() {
            events.insert(transition.event.clone());
        }
        
        // Built-in events
        events.insert("NullEvent".to_string());
        events.insert("EnterState".to_string());
        events.insert("ExitState".to_string());
        
        events
    }

    /// Get all state names from this state and its subregions
    pub fn states(&self, recursive: bool) -> Names {
        let mut states = Names::new();
        states.insert(self.id.clone());
        
        if recursive {
            for region in self.regions.values() {
                let region_states = region.borrow().states(recursive);
                states.extend(region_states);
            }
        }
        
        states
    }

    /// Get all region names from this state
    pub fn regions(&self, recursive: bool) -> Names {
        let mut regions = Names::new();
        
        for (id, region) in &self.regions {
            regions.insert(id.clone());
            if recursive {
                let sub_regions = region.borrow().regions(recursive);
                regions.extend(sub_regions);
            }
        }
        
        regions
    }

    /// Get initial states from this state and its subregions
    pub fn initial_states(&self) -> Names {
        let mut initial_states = Names::new();
        
        if self.is_initial {
            initial_states.insert(self.id.clone());
        }
        
        for region in self.regions.values() {
            let region_initial = region.borrow().initial_states();
            initial_states.extend(region_initial);
        }
        
        initial_states
    }
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for State {}

/// Region containing states
#[derive(Debug, Serialize, Deserialize)]
pub struct Region {
    pub id: Id,
    #[serde(skip)]
    pub owned_by_state: Option<WeakStateRef>,
    #[serde(skip)]
    pub substates: HashMap<Id, StateRef>,
    pub location: Location,
}

impl Region {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            owned_by_state: None,
            substates: HashMap::new(),
            location: Location::default(),
        }
    }

    /// Get all events from substates
    pub fn events(&self) -> Names {
        let mut events = Names::new();
        
        for state in self.substates.values() {
            let state_events = state.borrow().events();
            events.extend(state_events);
        }
        
        events
    }

    /// Get all state names from substates
    pub fn states(&self, recursive: bool) -> Names {
        let mut states = Names::new();
        
        for (id, state) in &self.substates {
            states.insert(id.clone());
            if recursive {
                let sub_states = state.borrow().states(recursive);
                states.extend(sub_states);
            }
        }
        
        states
    }

    /// Get all region names from substates
    pub fn regions(&self, recursive: bool) -> Names {
        let mut regions = Names::new();
        
        for state in self.substates.values() {
            let state_regions = state.borrow().regions(recursive);
            regions.extend(state_regions);
        }
        
        regions
    }

    /// Get initial states from substates
    pub fn initial_states(&self) -> Names {
        let mut initial_states = Names::new();
        
        for state in self.substates.values() {
            if state.borrow().is_initial {
                let state_initial = state.borrow().initial_states();
                initial_states.extend(state_initial);
            }
        }
        
        initial_states
    }

    /// Find a state by ID in this region or its substates
    pub fn find_state(&self, state_id: &str) -> Option<StateRef> {
        if let Some(state) = self.substates.get(state_id) {
            return Some(state.clone());
        }
        
        // Search in subregions
        for state in self.substates.values() {
            for region in state.borrow().regions.values() {
                if let Some(found) = region.borrow().find_state(state_id) {
                    return Some(found);
                }
            }
        }
        
        None
    }
}

impl PartialEq for Region {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Region {}

/// Configuration options for state machine code generation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StateMachineConfig {
    /// Any state can randomly receive any message
    pub all_random: bool,
}

/// Main state machine structure
#[derive(Debug, Serialize, Deserialize)]
pub struct StateMachine {
    pub id: Id,
    #[serde(skip)]
    pub regions: HashMap<Id, RegionRef>,
    pub config: StateMachineConfig,
    pub location: Location,
}

impl StateMachine {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            regions: HashMap::new(),
            config: StateMachineConfig::default(),
            location: Location::default(),
        }
    }

    pub fn set_id(&mut self, id: Id) {
        self.id = id;
    }

    /// Get all events across all regions and states
    pub fn events(&self) -> Names {
        let mut events = Names::new();
        
        for region in self.regions.values() {
            let region_events = region.borrow().events();
            events.extend(region_events);
        }
        
        events
    }

    /// Get all state names across all regions
    pub fn states(&self, recursive: bool) -> Names {
        let mut states = Names::new();
        
        for region in self.regions.values() {
            let region_states = region.borrow().states(recursive);
            states.extend(region_states);
        }
        
        states
    }

    /// Get all region names
    pub fn regions(&self, recursive: bool) -> Names {
        let mut regions = Names::new();
        
        for (id, region) in &self.regions {
            regions.insert(id.clone());
            if recursive {
                let sub_regions = region.borrow().regions(recursive);
                regions.extend(sub_regions);
            }
        }
        
        regions
    }

    /// Get all initial states
    pub fn initial_states(&self) -> Names {
        let mut initial_states = Names::new();
        
        for region in self.regions.values() {
            let region_initial = region.borrow().initial_states();
            initial_states.extend(region_initial);
        }
        
        initial_states
    }

    /// Find a state by ID anywhere in the state machine
    pub fn find_state(&self, state_id: &str) -> Option<StateRef> {
        for region in self.regions.values() {
            if let Some(state) = region.borrow().find_state(state_id) {
                return Some(state);
            }
        }
        None
    }

    /// Find which region contains a given state
    pub fn owner_region(&self, state_id: &str) -> Option<RegionRef> {
        for region in self.regions.values() {
            if region.borrow().substates.contains_key(state_id) {
                return Some(region.clone());
            }
            
            // Search in subregions
            for state in region.borrow().substates.values() {
                for sub_region in state.borrow().regions.values() {
                    if sub_region.borrow().find_state(state_id).is_some() {
                        return Some(sub_region.clone());
                    }
                }
            }
        }
        None
    }

    /// Get all final states
    pub fn final_states(&self) -> Names {
        let mut final_states = Names::new();
        
        for region in self.regions.values() {
            for state in region.borrow().substates.values() {
                let state_borrow = state.borrow();
                if state_borrow.is_final {
                    final_states.insert(state_borrow.id.clone());
                }
                
                // Check subregions recursively
                for sub_region in state_borrow.regions.values() {
                    for sub_state in sub_region.borrow().substates.values() {
                        if sub_state.borrow().is_final {
                            final_states.insert(sub_state.borrow().id.clone());
                        }
                    }
                }
            }
        }
        
        final_states
    }

    /// Get all transitions across all regions and states
    pub fn transitions(&self) -> Vec<Transition> {
        let mut transitions = Vec::new();
        
        for region in self.regions.values() {
            for state in region.borrow().substates.values() {
                let state_borrow = state.borrow();
                for transition in state_borrow.transitions.values() {
                    transitions.push(transition.clone());
                }
                
                // Get transitions from subregions recursively
                for sub_region in state_borrow.regions.values() {
                    for sub_state in sub_region.borrow().substates.values() {
                        let sub_state_borrow = sub_state.borrow();
                        for transition in sub_state_borrow.transitions.values() {
                            transitions.push(transition.clone());
                        }
                    }
                }
            }
        }
        
        transitions
    }

    /// Calculate the maximum depth of the state machine hierarchy
    pub fn depth(&self) -> usize {
        let mut max_depth = 0;
        
        for region in self.regions.values() {
            max_depth = max_depth.max(self.region_depth(&region.borrow()) + 1);
        }
        
        max_depth
    }

    #[allow(clippy::only_used_in_recursion)]
    fn region_depth(&self, region: &Region) -> usize {
        let mut max_depth = 0;
        
        for state in region.substates.values() {
            let state_borrow = state.borrow();
            if state_borrow.regions.is_empty() {
                max_depth = max_depth.max(1);
            } else {
                for sub_region in state_borrow.regions.values() {
                    max_depth = max_depth.max(self.region_depth(&sub_region.borrow()) + 1);
                }
            }
        }
        
        max_depth
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new("default".to_string())
    }
}