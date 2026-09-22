package WorkspaceAcceptance {
    part def ReviewedWorkspace;
    part def Reviewer;
    item def ReviewResult;
    requirement def StableRevision { subject stableWorkspace : ReviewedWorkspace; }

    case def ReviewCase {
        subject reviewWorkspace : ReviewedWorkspace :>> Cases::Case::subj;
        actor reviewer : Reviewer;
        objective preservation : StableRevision :>> Cases::Case::obj;
        return ref reviewResult : ReviewResult :>> Cases::Case::result;
    }

    verification def VerifyRevision {
        subject verifiedWorkspace : ReviewedWorkspace
            :>> VerificationCases::VerificationCase::subj;
        actor verifier : Reviewer;
        objective verificationObjective : StableRevision
            :>> VerificationCases::VerificationCase::obj;
        return verdict : VerificationCases::VerdictKind
            :>> VerificationCases::VerificationCase::verdict;
    }

    part def ReviewPlan {
        case plannedReview : ReviewCase;
        verification plannedVerification : VerifyRevision;
    }
}
