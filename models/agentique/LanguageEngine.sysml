package LanguageArchitecture {
    part def SemanticKernel {
        port kernelRevisions : ArchitectureContracts::ModelRevision;
    }
    part def KerMLEngine {
        ref part kernelContract : SemanticKernel;
        port kermlQueries : ArchitectureContracts::SemanticQuery;
    }
    part def SysMLEngine {
        ref part kermlContract : KerMLEngine;
        port sysmlQueries : ArchitectureContracts::SemanticQuery;
    }
    part def StandardLibraryManager {
        ref part libraryKernel : SemanticKernel;
        port publications : ArchitectureContracts::StandardPublication;
    }
    part def LanguageEngine {
        part semanticKernel : SemanticKernel;
        part kermlEngine : KerMLEngine;
        part sysmlEngine : SysMLEngine;
        part standardLibraries : StandardLibraryManager;
    }
}
