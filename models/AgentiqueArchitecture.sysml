package AgentiqueArchitecture {
    private import AgentiqueBehaviour::*;

    doc /* Logical self-model of Agentique, v0.1 draft.
       Agentique is the system under design. HumanOperator is external to it.
       Logical parts do not prescribe deployment processes or Rust structs.
       Interface connections below specify topology, not message routing or execution.
       Syntax and semantics require independent tool validation at M0.
    */

    item def EngineRequest;
    item def EngineResponse;

    port def EngineAccess {
        in item request : EngineRequest;
        out item response : EngineResponse;
    }

    part def HumanOperator;
    part def Conversation;
    part def ViewPanel;
    part def Surface {
        part views : ViewPanel[0..*];
    }
    part def Console {
        part conversation : Conversation;
        part surface : Surface;
        port access : ~EngineAccess;
    }
    part def Assistant {
        port tools : ~EngineAccess;
    }
    part def ModelService {
        exhibit state requestLifecycle : RequestLifecycle;
    }
    part def SimulationRuntime;
    part def WorkspaceStore;
    part def RunStore;

    part def Engine {
        part models : ModelService;
        part simulation : SimulationRuntime;
        part workspace : WorkspaceStore;
        part runs : RunStore;
        port operations : EngineAccess;
    }
    part def Agentique {
        part console : Console;
        part assistant : Assistant;
        part engine : Engine;
        connect console.access to engine.operations;
        connect assistant.tools to engine.operations;
    }

    part agentique : Agentique;

    part def OperatingContext {
        part operator : HumanOperator;
        part systemUnderDesign : Agentique;
    }
}
