package ArchitectureContracts {
    attribute def RevisionNumber;
    item def SemanticState;
    item def ValidatedSemanticState :> SemanticState;
    item def StandardArtifact;
    item def Diagnostic;
    item def ExecutionPlan;

    port def SemanticQuery { out item semanticAnswer : SemanticState; }
    port def ModelRevision { out item revisionState : SemanticState; }
    port def DiagnosticStream { out item diagnostic : Diagnostic; }
    port def StandardPublication { out item standardArtifact : StandardArtifact; }
    port def ExecutionModel { out item executionPlan : ExecutionPlan; }

    interface def SemanticAccess {
        end port requester : SemanticQuery;
        end port responder : SemanticQuery;
    }
}
