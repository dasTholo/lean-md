@lean-md
consumer: ai

@phase "pre-context"
@include hard-rules
@include dispatch-contract
@phase-end

@phase "explore"
Explore the problem space and surface the user's real intent. EXPLORE_PHASE_MARKER
@phase-end

@phase "handoff"
Hand the approved spec to the controller for plan-writing. HANDOFF_PHASE_MARKER
@phase-end
