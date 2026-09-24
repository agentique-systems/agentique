package PresentationAcceptance {
    view def ArchitectureView;
    viewpoint def ArchitecturePerspective;
    metadata def ReviewMetadata;
    #ReviewMetadata part def Presentation {
        view architecture : ArchitectureView;
        viewpoint perspective : ArchitecturePerspective;
    }
    part def SpecializedPresentation :> Presentation;
}
