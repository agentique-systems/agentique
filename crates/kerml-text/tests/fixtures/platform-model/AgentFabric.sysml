package AgentArchitecture {
    // Ordinary architecture concepts, not language extensions or executable behavior.
    part def AgentPolicy;
    part def AgentObservation;
    part def AgentAction;
    part def DecisionModel;
    part def Agent {
        ref part policy : AgentPolicy;
        ref part observation : AgentObservation;
        ref part proposedAction : AgentAction;
    }
    part def GenerativeAgent :> Agent;
    part def DecisionAgent :> Agent {
        ref part decisionProvider : DecisionModel;
    }
    part def AgentRuntime {
        ref part modelingService : PlatformArchitecture::ModelingService;
        part policy : AgentPolicy;
        part architectureAgent : GenerativeAgent;
        part viewDecision : DecisionAgent;
    }
}
