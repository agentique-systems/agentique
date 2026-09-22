package PlatformArchitecture {
    part def ProjectWorkspace {
        ref part kermlPublication : LanguageArchitecture::StandardLibraryManager;
        ref part systemsPublication : LanguageArchitecture::StandardLibraryManager;
        ref part languageQueries : LanguageArchitecture::SysMLEngine;
        attribute revisionNumber : ArchitectureContracts::RevisionNumber;
        port workspaceRevision : ArchitectureContracts::ModelRevision;
        port workspaceQuery : ArchitectureContracts::SemanticQuery;
        port workspaceDiagnostics : ArchitectureContracts::DiagnosticStream;
    }
    part def IncrementalWorkspace :> ProjectWorkspace {
        attribute acceptedRevision : ArchitectureContracts::RevisionNumber
            :>> ProjectWorkspace::revisionNumber;
    }
    part def QueryService {
        ref part queryKernel : LanguageArchitecture::SemanticKernel;
        port queryRead : ArchitectureContracts::SemanticQuery;
    }
    part def ValidationService {
        ref part validationLanguage : LanguageArchitecture::SysMLEngine;
        port validationDiagnostics : ArchitectureContracts::DiagnosticStream;
        out item validatedState : ArchitectureContracts::ValidatedSemanticState;
    }
    part def ViewService {
        ref part projectionQuery : QueryService;
        port viewQuery : ArchitectureContracts::SemanticQuery;
    }
    part def Repository {
        port repositoryRevisions : ArchitectureContracts::ModelRevision;
    }
    part def Client {
        port clientQuery : ArchitectureContracts::SemanticQuery;
    }
    action def ValidateRevision {
        in item workingState : ArchitectureContracts::SemanticState;
        out item checkedState : ArchitectureContracts::ValidatedSemanticState;
    }
    state def Serving {
        entry action startServing;
        do action checkRevision : ValidateRevision;
        exit action stopServing;
    }
    requirement def ImmutableRevisions {
        subject subjectWorkspace : ProjectWorkspace;
        require constraint preservesPriorState;
    }
    part def ModelingPlatform {
        part workspace : ProjectWorkspace;
        part queryService : QueryService;
        part validationService : ValidationService;
        part views : ViewService;
        part repository : Repository;
        part client : Client;
        port platformQueries : ArchitectureContracts::SemanticQuery;
        port clientQueries : ArchitectureContracts::SemanticQuery;
        interface queryConnection : ArchitectureContracts::SemanticAccess
            connect clientQueries to platformQueries;
        action validate : ValidateRevision;
        state operating : Serving;
    }
}
