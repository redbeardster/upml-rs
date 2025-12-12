use serde::{Deserialize, Serialize};
use crate::state_machine::{Id, Location, Guard, Effect, ActivityArgs};

/// Abstract Syntax Tree node types for PlantUML parsing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AstNode {
    Null,
    Transition(AstTransition),
    Activity(AstActivity),
    ConfigSetting(AstConfigSetting),
    State(AstState),
    Region(AstRegion),
    Machine(AstMachine),
}

/// Base trait for AST nodes with location information
pub trait AstNodeBase {
    fn location(&self) -> &Location;
    fn set_location(&mut self, location: Location);
}

/// Transition AST node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstTransition {
    pub id: Option<Id>,
    pub from_state: Id,
    pub to_state: Id,
    pub event: Id,
    pub guard: Guard,
    pub effect: Effect,
    pub location: Location,
}

impl AstNodeBase for AstTransition {
    fn location(&self) -> &Location {
        &self.location
    }
    
    fn set_location(&mut self, location: Location) {
        self.location = location;
    }
}

/// Activity AST node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstActivity {
    pub id: Option<Id>,
    pub state: Id,
    pub activity_type: Id,
    pub args: ActivityArgs,
    pub location: Location,
}

impl AstNodeBase for AstActivity {
    fn location(&self) -> &Location {
        &self.location
    }
    
    fn set_location(&mut self, location: Location) {
        self.location = location;
    }
}

/// Configuration setting AST node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstConfigSetting {
    pub state: Id,
    pub setting: String,
    pub location: Location,
}

impl AstNodeBase for AstConfigSetting {
    fn location(&self) -> &Location {
        &self.location
    }
    
    fn set_location(&mut self, location: Location) {
        self.location = location;
    }
}

/// State AST node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstState {
    pub id: Id,
    pub subtree: Vec<AstNode>,
    pub location: Location,
}

impl AstNodeBase for AstState {
    fn location(&self) -> &Location {
        &self.location
    }
    
    fn set_location(&mut self, location: Location) {
        self.location = location;
    }
}

/// Region AST node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstRegion {
    pub id: Option<Id>,
    pub subtree: Vec<AstNode>,
    pub location: Location,
}

impl AstNodeBase for AstRegion {
    fn location(&self) -> &Location {
        &self.location
    }
    
    fn set_location(&mut self, location: Location) {
        self.location = location;
    }
}

/// Machine AST node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstMachine {
    pub id: Option<Id>,
    pub subtree: Vec<AstNode>,
    pub location: Location,
}

impl AstNodeBase for AstMachine {
    fn location(&self) -> &Location {
        &self.location
    }
    
    fn set_location(&mut self, location: Location) {
        self.location = location;
    }
}

/// Visitor trait for traversing AST nodes
pub trait AstVisitor<T> {
    fn visit_transition(&mut self, node: &AstTransition) -> T;
    fn visit_activity(&mut self, node: &AstActivity) -> T;
    fn visit_config_setting(&mut self, node: &AstConfigSetting) -> T;
    fn visit_state(&mut self, node: &AstState) -> T;
    fn visit_region(&mut self, node: &AstRegion) -> T;
    fn visit_machine(&mut self, node: &AstMachine) -> T;
    
    fn visit(&mut self, node: &AstNode) -> T {
        match node {
            AstNode::Null => panic!("Cannot visit null node"),
            AstNode::Transition(t) => self.visit_transition(t),
            AstNode::Activity(a) => self.visit_activity(a),
            AstNode::ConfigSetting(c) => self.visit_config_setting(c),
            AstNode::State(s) => self.visit_state(s),
            AstNode::Region(r) => self.visit_region(r),
            AstNode::Machine(m) => self.visit_machine(m),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ast_node_creation() {
        let transition = AstTransition {
            id: Some("t1".to_string()),
            from_state: "State1".to_string(),
            to_state: "State2".to_string(),
            event: "Event1".to_string(),
            guard: vec!["x > 0".to_string()],
            effect: vec!["action()".to_string()],
            location: Location::default(),
        };
        
        let node = AstNode::Transition(transition);
        
        match node {
            AstNode::Transition(t) => {
                assert_eq!(t.from_state, "State1");
                assert_eq!(t.to_state, "State2");
                assert_eq!(t.event, "Event1");
            }
            _ => panic!("Expected transition node"),
        }
    }
}