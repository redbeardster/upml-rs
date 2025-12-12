/*
 * Enhanced version of generated Promela FSM model
 * with event generator for testing
 */

/* Message types (events) */
mtype = {
    EnterState,
    Event1,
    ExitState,
    NullEvent
};

/* State enumeration */
mtype = {
    state_State1,
    state_State2
};

/* Global variables */
mtype currentState;
mtype event;
chan eventQueue = [16] of { mtype };
bool inTransition = false;

/* Event generator process for testing */
active proctype EventGenerator() {
    do
    :: eventQueue!Event1;
    :: eventQueue!NullEvent;
    :: skip; /* Allow process to terminate */
    od
}

/* Main FSM process */
active proctype StateMachine() {
    currentState = state_State1; 
    
    do
    :: eventQueue?event ->
        atomic {
            inTransition = true;
            
            /* State transitions */
            if
            :: (currentState == state_State1 && event == Event1) ->
                    currentState = state_State2;
            :: else -> skip
            fi
            if
            :: (currentState == state_State2 && event == NullEvent) ->
                    goto end_state;
            :: else -> skip
            fi
            
            inTransition = false;
        }
    od
    
end_state:
    /* Final state reached - process terminates */
}

/* LTL Properties */
/* Basic liveness: eventually reach a final state */
ltl liveness { <>( currentState == state_State2 ) }
ltl safety { [](!inTransition -> <>inTransition) }