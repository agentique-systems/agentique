package AgentiqueSystem {
    part def Agentique {
        part languageSubsystem : LanguageArchitecture::LanguageEngine;
        part modeling : PlatformArchitecture::ModelingPlatform;
        part execution : ExecutionArchitecture::ExecutionSubsystem;
    }
}
