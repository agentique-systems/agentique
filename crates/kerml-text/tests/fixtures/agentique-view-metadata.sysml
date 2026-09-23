package PresentationAcceptance {
    view def ArchitectureView;
    metadata def ReviewMetadata;
    part def Presentation {
        view architecture : ArchitectureView;
        metadata review : ReviewMetadata;
    }
    part def SpecializedPresentation :> Presentation;
}
