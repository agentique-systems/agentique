package ExecutionArchitecture {
    part def ExecutionCompiler {
        ref part validationContract : PlatformArchitecture::ValidationService;
        in item validatedInput : ArchitectureContracts::ValidatedSemanticState;
        port executionOutput : ArchitectureContracts::ExecutionModel;
    }
    part def ExecutionIR {
        item compiledPlan : ArchitectureContracts::ExecutionPlan;
    }
    part def SimulationRuntime {
        ref part irContract : ExecutionIR;
        port executionInput : ArchitectureContracts::ExecutionModel;
    }
    part def ExecutionSubsystem {
        part compiler : ExecutionCompiler;
        part executionIR : ExecutionIR;
        part simulation : SimulationRuntime;
    }
}
