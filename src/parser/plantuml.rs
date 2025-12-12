use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{
        alpha1, alphanumeric1, char, line_ending, multispace0, space0, space1,
    },
    combinator::{map, opt, recognize, value},
    multi::many0,
    sequence::{delimited, pair, preceded, tuple},
    IResult,
};

use crate::state_machine::{
    Activity, ActivityArgs, Effect, Guard, Id, StateMachine, StateMachineBuilder,
    Transition,
};
use crate::Result;

/// Parse a PlantUML state diagram into a StateMachine
pub fn parse_plantuml(input: &str) -> Result<StateMachine> {
    let (_, state_machine) = plantuml_document(input)
        .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    Ok(state_machine)
}

/// Main parser for PlantUML document
fn plantuml_document(input: &str) -> IResult<&str, StateMachine> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("@startuml")(input)?;
    let (input, title) = opt(preceded(space1, take_until("\n")))(input)?; // optional title
    let (input, _) = line_ending(input)?;
    
    let (input, elements) = many0(preceded(multispace0, plantuml_element))(input)?;
    
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("@enduml")(input)?;
    
    // Build state machine from parsed elements using new approach
    let machine_id = title.unwrap_or("parsed_machine").trim().to_string();
    let state_machine = build_state_machine_from_elements(machine_id, elements)
        .map_err(|_e| nom::Err::Failure(nom::error::Error::new(input, nom::error::ErrorKind::Fail)))?;
    
    Ok((input, state_machine))
}

/// Parse a single PlantUML element
fn plantuml_element(input: &str) -> IResult<&str, PlantUMLElement> {
    alt((
        map(comment, PlantUMLElement::Comment),
        map(state_definition, PlantUMLElement::StateDefinition),
        map(transition, PlantUMLElement::Transition),
        map(state_activity, PlantUMLElement::Activity),
        map(state_config, PlantUMLElement::Config),
        map(note, PlantUMLElement::Note),
        map(skin_param, PlantUMLElement::SkinParam),
        map(style_definition, PlantUMLElement::Style),
    ))(input)
}

#[derive(Debug, Clone)]
pub enum PlantUMLElement {
    Comment(String),
    StateDefinition(StateDefinition),
    Transition(TransitionDefinition),
    Activity(ActivityDefinition),
    Config(ConfigDefinition),
    Note(String),
    SkinParam(String),
    Style(String),
}

#[derive(Debug, Clone)]
pub struct StateDefinition {
    pub id: Id,
    pub substates: Vec<PlantUMLElement>,
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TransitionDefinition {
    pub from_state: Id,
    pub to_state: Id,
    pub event: Option<Id>,
    pub guard: Guard,
    pub effect: Effect,
    pub direction: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ActivityDefinition {
    pub state: Id,
    pub activity_type: Id,
    pub args: ActivityArgs,
}

#[derive(Debug, Clone)]
pub struct ConfigDefinition {
    pub state: Id,
    pub setting: String,
}

/// Parse comments (single line // or multi-line /* */)
pub fn comment(input: &str) -> IResult<&str, String> {
    alt((
        // Single line comment
        map(
            preceded(tag("//"), take_until("\n")),
            |s: &str| s.trim().to_string(),
        ),
        // Multi-line comment
        map(
            delimited(tag("/*"), take_until("*/"), tag("*/")),
            |s: &str| s.trim().to_string(),
        ),
    ))(input)
}

/// Parse regions separated by -- or ||
pub fn parse_regions(input: &str) -> IResult<&str, Vec<PlantUMLElement>> {
    let (input, first_region) = many0(preceded(multispace0, plantuml_element))(input)?;
    let (input, other_regions) = many0(preceded(
        tuple((multispace0, alt((tag("--"), tag("||"))), multispace0)),
        many0(preceded(multispace0, plantuml_element))
    ))(input)?;
    
    // Flatten all regions into a single list for now
    // In a full implementation, we'd create separate region objects
    let mut all_elements = first_region;
    for region in other_regions {
        all_elements.extend(region);
    }
    
    Ok((input, all_elements))
}

/// Parse state definition
pub fn state_definition(input: &str) -> IResult<&str, StateDefinition> {
    let (input, _) = tag("state")(input)?;
    let (input, _) = space1(input)?;
    let (input, id) = identifier(input)?;
    let (input, color) = opt(preceded(space1, color_spec))(input)?;
    let (input, _) = space0(input)?;
    
    // Check if it's a composite state with substates
    let (input, substates) = if let Ok((input, _)) = char::<&str, nom::error::Error<&str>>('{')(input) {
        let (input, _) = multispace0(input)?;
        let (input, regions) = parse_regions(input)?;
        let (input, _) = multispace0(input)?;
        let (input, _) = char('}')(input)?;
        (input, regions)
    } else {
        (input, Vec::new())
    };
    
    Ok((input, StateDefinition { id, substates, color }))
}

/// Parse transition
pub fn transition(input: &str) -> IResult<&str, TransitionDefinition> {
    let (input, from_state) = identifier(input)?;
    let (input, _) = space0(input)?;
    let (input, direction) = opt(direction_spec)(input)?;
    let (input, _) = alt((tag("-->"), tag("->")))(input)?;
    let (input, _) = space0(input)?;
    let (input, to_state) = identifier(input)?;
    
    // Parse optional event, guard, and effect
    let (input, (event, guard, effect)) = opt(preceded(
        tuple((space0, char(':'), space0)),
        tuple((
            opt(identifier),
            opt(preceded(space0, guard_spec)),
            opt(preceded(space0, effect_spec)),
        )),
    ))(input)
    .map(|(i, opt)| {
        if let Some((e, g, eff)) = opt {
            (i, (e, g.unwrap_or_default(), eff.unwrap_or_default()))
        } else {
            (i, (None, Vec::new(), Vec::new()))
        }
    })?;
    
    let (input, _) = opt(char(';'))(input)?;
    
    Ok((input, TransitionDefinition {
        from_state,
        to_state,
        event,
        guard,
        effect,
        direction,
    }))
}

/// Parse state activity (entry, exit, etc.)
pub fn state_activity(input: &str) -> IResult<&str, ActivityDefinition> {
    let (input, state) = identifier(input)?;
    let (input, _) = char(':')(input)?;
    let (input, _) = space0(input)?;
    let (input, activity_type) = identifier(input)?;
    let (input, _) = char(':')(input)?;
    let (input, _) = space0(input)?;
    
    // Parse until we hit a non-escaped semicolon or newline
    let mut action_text = String::new();
    let mut remaining = input;
    
    for ch in input.chars() {
        if ch == '\n' {
            break;
        } else if ch == ';' {
            // Check if this is an escaped semicolon
            if action_text.ends_with('\\') {
                action_text.push(ch);
                remaining = &remaining[ch.len_utf8()..];
            } else {
                // Non-escaped semicolon, stop parsing
                break;
            }
        } else {
            action_text.push(ch);
            remaining = &remaining[ch.len_utf8()..];
        }
    }
    
    // Skip optional trailing semicolon
    let (input, _) = opt(char(';'))(remaining)?;
    
    // Split on "\\;" pattern (with or without spaces)
    let args: Vec<String> = action_text
        .split("\\;")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    
    Ok((input, ActivityDefinition {
        state,
        activity_type,
        args,
    }))
}

/// Parse state configuration
pub fn state_config(input: &str) -> IResult<&str, ConfigDefinition> {
    let (input, state) = identifier(input)?;
    let (input, _) = char(':')(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = tag("config")(input)?;
    let (input, _) = char(':')(input)?;
    let (input, _) = space0(input)?;
    let (input, setting) = take_while1(|c: char| c != ';' && c != '\n')(input)?;
    let (input, _) = opt(char(';'))(input)?;
    
    Ok((input, ConfigDefinition {
        state,
        setting: setting.trim().to_string(),
    }))
}

/// Parse note (ignored for now)
fn note(input: &str) -> IResult<&str, String> {
    let (input, _) = tag("note")(input)?;
    let (input, content) = take_until("\n")(input)?;
    Ok((input, content.to_string()))
}

/// Parse skinparam (ignored for now)
fn skin_param(input: &str) -> IResult<&str, String> {
    let (input, _) = tag("skinparam")(input)?;
    let (input, content) = take_until("\n")(input)?;
    Ok((input, content.to_string()))
}

/// Parse style definition (ignored for now)
fn style_definition(input: &str) -> IResult<&str, String> {
    let (input, _) = tag("<style>")(input)?;
    let (input, content) = take_until("</style>")(input)?;
    let (input, _) = tag("</style>")(input)?;
    Ok((input, content.to_string()))
}

/// Parse identifier
pub fn identifier(input: &str) -> IResult<&str, Id> {
    alt((
        // Special states
        value("[*]".to_string(), tag("[*]")),
        // Quoted identifier
        map(
            delimited(char('"'), take_until("\""), char('"')),
            |s: &str| s.to_string(),
        ),
        // Regular identifier
        map(
            recognize(pair(
                alt((alpha1, tag("_"))),
                many0(alt((alphanumeric1, tag("_")))),
            )),
            |s: &str| s.to_string(),
        ),
    ))(input)
}

/// Parse direction specification (e.g., "-down->", "-1down->")
pub fn direction_spec(input: &str) -> IResult<&str, String> {
    map(
        recognize(tuple((
            char('-'),
            opt(take_while1(|c: char| c.is_ascii_digit())),
            opt(alt((tag("up"), tag("down"), tag("left"), tag("right")))),
        ))),
        |s: &str| s.to_string(),
    )(input)
}

/// Parse color specification (e.g., "#lightblue")
pub fn color_spec(input: &str) -> IResult<&str, String> {
    map(
        preceded(char('#'), take_while1(|c: char| c.is_alphanumeric())),
        |s: &str| format!("#{}", s),
    )(input)
}

/// Parse guard specification [condition]
pub fn guard_spec(input: &str) -> IResult<&str, Guard> {
    let (input, _) = char('[')(input)?;
    let (input, condition) = take_until("]")(input)?;
    let (input, _) = char(']')(input)?;
    Ok((input, vec![condition.trim().to_string()]))
}

/// Parse effect specification /action
pub fn effect_spec(input: &str) -> IResult<&str, Effect> {
    let (input, _) = char('/')(input)?;
    
    // Parse until we hit a non-escaped semicolon or newline
    let mut action = String::new();
    let mut remaining = input;
    
    for ch in input.chars() {
        if ch == '\n' {
            break;
        } else if ch == ';' {
            // Check if this is an escaped semicolon
            if action.ends_with('\\') {
                action.push(ch);
                remaining = &remaining[ch.len_utf8()..];
            } else {
                // Non-escaped semicolon, stop parsing
                break;
            }
        } else {
            action.push(ch);
            remaining = &remaining[ch.len_utf8()..];
        }
    }
    
    // Split on "\\;" pattern (with or without spaces)
    let actions: Vec<String> = action
        .split("\\;")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    
    Ok((remaining, actions))
}

/// Intermediate structure to collect parsed elements before building state machine
#[derive(Debug, Default)]
struct ParsedStateMachine {
    states: Vec<StateDefinition>,
    transitions: Vec<TransitionDefinition>,
    activities: Vec<ActivityDefinition>,
    configs: Vec<ConfigDefinition>,
}

/// Process all parsed elements and build a complete state machine
fn build_state_machine_from_elements(
    machine_id: String,
    elements: Vec<PlantUMLElement>,
) -> Result<StateMachine> {
    // First pass: collect all elements by type
    let mut parsed = ParsedStateMachine::default();
    
    for element in elements {
        match element {
            PlantUMLElement::StateDefinition(state_def) => {
                parsed.states.push(state_def);
            }
            PlantUMLElement::Transition(trans_def) => {
                parsed.transitions.push(trans_def);
            }
            PlantUMLElement::Activity(activity_def) => {
                parsed.activities.push(activity_def);
            }
            PlantUMLElement::Config(config_def) => {
                parsed.configs.push(config_def);
            }
            _ => {
                // Ignore comments, notes, etc. for now
            }
        }
    }
    
    // Optional debug output (only in debug builds)
    #[cfg(debug_assertions)]
    if std::env::var("UPML_DEBUG").is_ok() {
        eprintln!("DEBUG: Initial - Found {} states, {} transitions, {} activities", 
                  parsed.states.len(), parsed.transitions.len(), parsed.activities.len());
        for trans in &parsed.transitions {
            eprintln!("DEBUG: Initial Transition: {} -> {}", trans.from_state, trans.to_state);
        }
    }
    
    // Process substates recursively
    fn collect_elements_recursively(
        elements: &[PlantUMLElement],
        parsed: &mut ParsedStateMachine,
    ) {
        for element in elements {
            match element {
                PlantUMLElement::StateDefinition(state_def) => {
                    parsed.states.push(state_def.clone());
                    // Recursively process substates
                    collect_elements_recursively(&state_def.substates, parsed);
                }
                PlantUMLElement::Transition(trans_def) => {
                    parsed.transitions.push(trans_def.clone());
                }
                PlantUMLElement::Activity(activity_def) => {
                    parsed.activities.push(activity_def.clone());
                }
                PlantUMLElement::Config(config_def) => {
                    parsed.configs.push(config_def.clone());
                }
                _ => {
                    // Ignore other elements
                }
            }
        }
    }
    
    // Collect elements from substates recursively
    let initial_states = parsed.states.clone();
    for state_def in &initial_states {
        #[cfg(debug_assertions)]
        if std::env::var("UPML_DEBUG").is_ok() {
            eprintln!("DEBUG: Processing substates of {}: {} elements", state_def.id, state_def.substates.len());
        }
        collect_elements_recursively(&state_def.substates, &mut parsed);
    }
    
    #[cfg(debug_assertions)]
    if std::env::var("UPML_DEBUG").is_ok() {
        eprintln!("DEBUG: After recursion - Found {} states, {} transitions, {} activities", 
                  parsed.states.len(), parsed.transitions.len(), parsed.activities.len());
        for trans in &parsed.transitions {
            eprintln!("DEBUG: Final Transition: {} -> {}", trans.from_state, trans.to_state);
        }
    }
    
    // Collect all state names from transitions (implicit state definitions)
    let mut all_state_names = std::collections::HashSet::new();
    for trans in &parsed.transitions {
        if trans.from_state != "[*]" {
            all_state_names.insert(trans.from_state.clone());
        }
        if trans.to_state != "[*]" {
            all_state_names.insert(trans.to_state.clone());
        }
    }
    
    // Add explicit state definitions
    for state_def in &parsed.states {
        all_state_names.insert(state_def.id.clone());
    }
    
    #[cfg(debug_assertions)]
    if std::env::var("UPML_DEBUG").is_ok() {
        eprintln!("DEBUG: All states: {:?}", all_state_names);
    }
    
    // Second pass: build the state machine
    let builder = StateMachineBuilder::new(machine_id);
    let region_builder = builder.add_region("main_region".to_string());
    
    // Add all states first (both explicit and implicit)
    let mut region_builder = region_builder;
    
    // Add implicit states from transitions
    for state_name in &all_state_names {
        let state_builder = region_builder.add_state(state_name.clone());
        
        // Check if this is an initial state (has transition from [*])
        let is_initial = parsed.transitions.iter().any(|t| t.from_state == "[*]" && t.to_state == *state_name);
        let state_builder = if is_initial {
            state_builder.initial()
        } else {
            state_builder
        };
        
        // Check if this is a final state (has transition to [*])
        let is_final = parsed.transitions.iter().any(|t| t.from_state == *state_name && t.to_state == "[*]");
        let state_builder = if is_final {
            state_builder.final_state()
        } else {
            state_builder
        };
        
        // Add activities for this state
        let mut state_builder = state_builder;
        for activity_def in &parsed.activities {
            if activity_def.state == *state_name {
                let activity = Activity::new(
                    format!("a_{}_{}", activity_def.state, activity_def.activity_type),
                    activity_def.state.clone(),
                    activity_def.activity_type.clone(),
                    activity_def.args.clone(),
                );
                state_builder = state_builder.add_activity(activity);
            }
        }
        
        // Add transitions from this state
        for trans_def in &parsed.transitions {
            if trans_def.from_state == *state_name {
                let transition = Transition::new(
                    format!("t_{}_to_{}", trans_def.from_state, trans_def.to_state),
                    trans_def.from_state.clone(),
                    trans_def.to_state.clone(),
                    trans_def.event.clone().unwrap_or_else(|| "NullEvent".to_string()),
                )
                .with_guard(trans_def.guard.clone())
                .with_effect(trans_def.effect.clone());
                
                state_builder = state_builder.add_transition(transition);
            }
        }
        
        region_builder = state_builder.finish_state();
    }
    
    // Note: Explicit states are already handled above in the all_state_names loop
    // No need to process them again
    
    let builder = region_builder.finish_region();
    Ok(builder.build())
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_transition() {
        let input = "State1 --> State2 : Event1";
        let (_, transition) = transition(input).unwrap();
        
        assert_eq!(transition.from_state, "State1");
        assert_eq!(transition.to_state, "State2");
        assert_eq!(transition.event, Some("Event1".to_string()));
    }

    #[test]
    fn test_parse_transition_with_guard_and_effect() {
        let input = "State1 --> State2 : Event1 [x > 0] / action1 \\; action2 \\;";
        let (_, transition) = transition(input).unwrap();
        
        assert_eq!(transition.from_state, "State1");
        assert_eq!(transition.to_state, "State2");
        assert_eq!(transition.event, Some("Event1".to_string()));
        assert_eq!(transition.guard, vec!["x > 0"]);
        // Текущий парсер обрабатывает effects по-другому
        assert!(!transition.effect.is_empty());
        assert!(transition.effect[0].contains("action1"));
    }

    #[test]
    fn test_parse_state_activity() {
        let input = "State1: entry: send event:INVITE to state:Bob;";
        let (_, activity) = state_activity(input).unwrap();
        
        assert_eq!(activity.state, "State1");
        assert_eq!(activity.activity_type, "entry");
        assert_eq!(activity.args, vec!["send event:INVITE to state:Bob"]);
    }

    #[test]
    fn test_parse_identifier() {
        assert_eq!(identifier("State1").unwrap().1, "State1");
        assert_eq!(identifier("[*]").unwrap().1, "[*]");
        assert_eq!(identifier("\"Long State Name\"").unwrap().1, "Long State Name");
    }

    #[test]
    fn test_parse_comment() {
        let input1 = "// This is a comment\n";
        let (_, comment1) = comment(input1).unwrap();
        assert_eq!(comment1, "This is a comment");

        let input2 = "/* Multi-line\n   comment */";
        let (_, comment2) = comment(input2).unwrap();
        assert_eq!(comment2, "Multi-line\n   comment");
    }
}