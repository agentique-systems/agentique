package AgentiqueVerification {
    private import AgentiqueArchitecture::*;
    private import AgentiqueRequirements::*;
    doc /* Verification declarations name obligations only. They have no implemented procedures
       or computed verdicts in this starter package. See requirements.json for acceptance procedures. */

    verification def verify_std01 {
        subject systemUnderTest : Agentique;
        objective {
            verify std01;
        }
    }

    verification def verify_std02 {
        subject systemUnderTest : Agentique;
        objective {
            verify std02;
        }
    }

    verification def verify_self01 {
        subject systemUnderTest : Agentique;
        objective {
            verify self01;
        }
    }

    verification def verify_mod01 {
        subject systemUnderTest : Agentique;
        objective {
            verify mod01;
        }
    }

    verification def verify_mod02 {
        subject systemUnderTest : Agentique;
        objective {
            verify mod02;
        }
    }

    verification def verify_mod03 {
        subject systemUnderTest : Agentique;
        objective {
            verify mod03;
        }
    }

    verification def verify_sim01 {
        subject systemUnderTest : Agentique;
        objective {
            verify sim01;
        }
    }

    verification def verify_sim02 {
        subject systemUnderTest : Agentique;
        objective {
            verify sim02;
        }
    }

    verification def verify_sim03 {
        subject systemUnderTest : Agentique;
        objective {
            verify sim03;
        }
    }

    verification def verify_sim04 {
        subject systemUnderTest : Agentique;
        objective {
            verify sim04;
        }
    }

    verification def verify_sim05 {
        subject systemUnderTest : Agentique;
        objective {
            verify sim05;
        }
    }

    verification def verify_sim06 {
        subject systemUnderTest : Agentique;
        objective {
            verify sim06;
        }
    }

    verification def verify_ui01 {
        subject systemUnderTest : Agentique;
        objective {
            verify ui01;
        }
    }

    verification def verify_ui02 {
        subject systemUnderTest : Agentique;
        objective {
            verify ui02;
        }
    }

    verification def verify_ai01 {
        subject systemUnderTest : Agentique;
        objective {
            verify ai01;
        }
    }

    verification def verify_sec01 {
        subject systemUnderTest : Agentique;
        objective {
            verify sec01;
        }
    }

    verification def verify_nfr01 {
        subject systemUnderTest : Agentique;
        objective {
            verify nfr01;
        }
    }

    verification def verify_nfr02 {
        subject systemUnderTest : Agentique;
        objective {
            verify nfr02;
        }
    }

    verification def verify_ext01 {
        subject systemUnderTest : Agentique;
        objective {
            verify ext01;
        }
    }

    verification def verify_trace01 {
        subject systemUnderTest : Agentique;
        objective {
            verify trace01;
        }
    }

}
