package PresentationAcceptance {
    view def ArchitectureView;
    metadata def ReviewMetadata;
    #ReviewMetadata part def Presentation {
        view architecture : ArchitectureView;
    }
    part def SpecializedPresentation :> Presentation;
}
