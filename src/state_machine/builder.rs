
use std::rc::Rc;
use std::cell::RefCell;

use super::types::*;

/// Builder for constructing state machines
pub struct StateMachineBuilder {
    state_machine: StateMachine,
}

impl StateMachineBuilder {
    pub fn new(id: Id) -> Self {
        Self {
            state_machine: StateMachine::new(id),
        }
    }

    pub fn with_config(mut self, config: StateMachineConfig) -> Self {
        self.state_machine.config = config;
        self
    }

    pub fn add_region(mut self, region_id: Id) -> RegionBuilder {
        let region = Rc::new(RefCell::new(Region::new(region_id.clone())));
        self.state_machine.regions.insert(region_id, region.clone());
        
        RegionBuilder {
            state_machine_builder: self,
            region,
        }
    }

    pub fn build(self) -> StateMachine {
        self.state_machine
    }
}

/// Builder for constructing regions
pub struct RegionBuilder {
    state_machine_builder: StateMachineBuilder,
    region: RegionRef,
}

impl RegionBuilder {
    pub fn add_state(self, state_id: Id) -> StateBuilder {
        let state = Rc::new(RefCell::new(State::new(state_id.clone())));
        
        // Set up bidirectional references
        state.borrow_mut().owned_by_region = Some(Rc::downgrade(&self.region));
        self.region.borrow_mut().substates.insert(state_id, state.clone());
        
        StateBuilder {
            region_builder: self,
            state,
        }
    }

    pub fn finish_region(self) -> StateMachineBuilder {
        self.state_machine_builder
    }
}

/// Builder for constructing states
pub struct StateBuilder {
    region_builder: RegionBuilder,
    state: StateRef,
}

impl StateBuilder {
    pub fn initial(self) -> Self {
        self.state.borrow_mut().is_initial = true;
        self
    }

    pub fn final_state(self) -> Self {
        self.state.borrow_mut().is_final = true;
        self
    }

    pub fn with_config(self, config: StateConfig) -> Self {
        self.state.borrow_mut().config = config;
        self
    }

    pub fn add_transition(self, transition: Transition) -> Self {
        let transition_id = transition.id.clone();
        self.state.borrow_mut().transitions.insert(transition_id, transition);
        self
    }

    pub fn add_activity(self, activity: Activity) -> Self {
        self.state.borrow_mut().activities.push(activity);
        self
    }

    pub fn add_subregion(self, region_id: Id) -> SubRegionBuilder {
        let region = Rc::new(RefCell::new(Region::new(region_id.clone())));
        
        // Set up bidirectional reference
        region.borrow_mut().owned_by_state = Some(Rc::downgrade(&self.state));
        self.state.borrow_mut().regions.insert(region_id, region.clone());
        
        SubRegionBuilder {
            state_builder: self,
            region,
        }
    }

    pub fn finish_state(self) -> RegionBuilder {
        self.region_builder
    }
}

/// Builder for constructing subregions within states
pub struct SubRegionBuilder {
    state_builder: StateBuilder,
    region: RegionRef,
}

impl SubRegionBuilder {
    pub fn add_state(self, state_id: Id) -> SubStateBuilder {
        let state = Rc::new(RefCell::new(State::new(state_id.clone())));
        
        // Set up bidirectional references
        state.borrow_mut().owned_by_region = Some(Rc::downgrade(&self.region));
        state.borrow_mut().super_state = Some(Rc::downgrade(&self.state_builder.state));
        self.region.borrow_mut().substates.insert(state_id, state.clone());
        
        SubStateBuilder {
            subregion_builder: self,
            state,
        }
    }

    pub fn finish_subregion(self) -> StateBuilder {
        self.state_builder
    }
}

/// Builder for constructing substates within subregions
pub struct SubStateBuilder {
    subregion_builder: SubRegionBuilder,
    state: StateRef,
}

impl SubStateBuilder {
    pub fn initial(self) -> Self {
        self.state.borrow_mut().is_initial = true;
        self
    }

    pub fn final_state(self) -> Self {
        self.state.borrow_mut().is_final = true;
        self
    }

    pub fn with_config(self, config: StateConfig) -> Self {
        self.state.borrow_mut().config = config;
        self
    }

    pub fn add_transition(self, transition: Transition) -> Self {
        let transition_id = transition.id.clone();
        self.state.borrow_mut().transitions.insert(transition_id, transition);
        self
    }

    pub fn add_activity(self, activity: Activity) -> Self {
        self.state.borrow_mut().activities.push(activity);
        self
    }

    pub fn finish_substate(self) -> SubRegionBuilder {
        self.subregion_builder
    }
}

/// Helper functions for creating common structures
impl Transition {
    pub fn new(
        id: Id,
        from_state: Id,
        to_state: Id,
        event: Id,
    ) -> Self {
        Self {
            id,
            from_state,
            to_state,
            event,
            guard: Vec::new(),
            effect: Vec::new(),
            location: Location::default(),
        }
    }

    pub fn with_guard(mut self, guard: Guard) -> Self {
        self.guard = guard;
        self
    }

    pub fn with_effect(mut self, effect: Effect) -> Self {
        self.effect = effect;
        self
    }

    pub fn with_location(mut self, location: Location) -> Self {
        self.location = location;
        self
    }
}

impl Activity {
    pub fn new(
        id: Id,
        state: Id,
        activity_type: Id,
        args: ActivityArgs,
    ) -> Self {
        Self {
            id,
            state,
            activity_type,
            args,
            location: Location::default(),
        }
    }

    pub fn with_location(mut self, location: Location) -> Self {
        self.location = location;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_machine_builder() {
        let sm = StateMachineBuilder::new("test_machine".to_string())
            .add_region("main_region".to_string())
                .add_state("state1".to_string())
                    .initial()
                    .add_transition(Transition::new(
                        "t1".to_string(),
                        "state1".to_string(),
                        "state2".to_string(),
                        "event1".to_string(),
                    ))
                    .finish_state()
                .add_state("state2".to_string())
                    .final_state()
                    .finish_state()
                .finish_region()
            .build();

        assert_eq!(sm.id, "test_machine");
        assert_eq!(sm.regions.len(), 1);
        
        let region = sm.regions.get("main_region").unwrap();
        assert_eq!(region.borrow().substates.len(), 2);
        
        let state1 = region.borrow().substates.get("state1").unwrap().clone();
        assert!(state1.borrow().is_initial);
        assert_eq!(state1.borrow().transitions.len(), 1);
        
        let state2 = region.borrow().substates.get("state2").unwrap().clone();
        assert!(state2.borrow().is_final);
    }
}