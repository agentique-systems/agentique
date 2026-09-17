package AgentiqueRequirements {
    private import AgentiqueArchitecture::*;
    doc /* Documentary requirements: acceptance procedures are in the specification and requirements.json.
       No automatic satisfaction is asserted. Verification bodies are deliberately not executable.
       CI must bind these declarations to real tests and evidence, not evaluate empty bodies as success. */

    requirement <'AGQ-STD01'> std01 {
        subject systemUnderTest : Agentique;
        doc /* Use the pinned KerML 1.0 and SysML 2.0 definitions for every supported language construct; identify relevant clauses, metaclasses, library dependencies, and tests. */
    }

    requirement <'AGQ-STD02'> std02 {
        subject systemUnderTest : Agentique;
        doc /* Publish separate support status for parsing, representation, validation, interchange, and execution. Reject unsupported semantics with a diagnostic; do not silently approximate them. */
    }

    requirement <'AGQ-SELF01'> self01 {
        subject systemUnderTest : Agentique;
        doc /* Maintain Agentique architecture, requirements, a runnable behaviour slice, and verification definitions in version-controlled SysML, linked to implementation modules and tests. */
    }

    requirement <'AGQ-MOD01'> mod01 {
        subject systemUnderTest : Agentique;
        doc /* Assign stable model-element identities and preserve references through supported rename, move, edit, and persistence operations. Runtime occurrence identity shall remain separate. */
    }

    requirement <'AGQ-MOD02'> mod02 {
        subject systemUnderTest : Agentique;
        doc /* Apply model edits as atomic change sets against an explicit base revision. Preserve invalid drafts separately from the last accepted revision. */
    }

    requirement <'AGQ-MOD03'> mod03 {
        subject systemUnderTest : Agentique;
        doc /* Read and write the selected standard textual and project-interchange representations for the supported subset, preserving meaning and explicit identity mappings. */
    }

    requirement <'AGQ-SIM01'> sim01 {
        subject systemUnderTest : Agentique;
        doc /* Prepare execution only after reference resolution, required validation, complete scenario binding, and supported-semantics checks have succeeded. */
    }

    requirement <'AGQ-SIM02'> sim02 {
        subject systemUnderTest : Agentique;
        doc /* Create isolated run state from immutable model and scenario revisions. A run shall not mutate the authored model or another run. */
    }

    requirement <'AGQ-SIM03'> sim03 {
        subject systemUnderTest : Agentique;
        doc /* Support initialise, run, pause, step, stop, reset-as-new-run, and inspection under the AGQ-SEQ-01 execution contract. */
    }

    requirement <'AGQ-SIM04'> sim04 {
        subject systemUnderTest : Agentique;
        doc /* Reproduce the semantic trace for identical model, libraries, scenario, engine build, and execution-contract versions. Exclude wall-clock and generated identifiers from trace comparison. */
    }

    requirement <'AGQ-SIM05'> sim05 {
        subject systemUnderTest : Agentique;
        doc /* Distinguish a completed experiment from a passed check, and distinguish waiting, exhausted inputs, ambiguity, limits, interruption, and evaluation errors. */
    }

    requirement <'AGQ-SIM06'> sim06 {
        subject systemUnderTest : Agentique;
        doc /* Record each input delivery and semantic step with its model reference, run identity, ordering, before/after state references, and evaluation outcome. */
    }

    requirement <'AGQ-UI01'> ui01 {
        subject systemUnderTest : Agentique;
        doc /* Use one accepted model revision and explicit run context across the Conversation and Surface. All semantic writes shall use the same Engine operation boundary. */
    }

    requirement <'AGQ-UI02'> ui02 {
        subject systemUnderTest : Agentique;
        doc /* Provide a Canvas of model-backed Views for structure, properties, behaviour, and run history. Keep layout independent of model semantics; provide keyboard and non-diagram alternatives. */
    }

    requirement <'AGQ-AI01'> ai01 {
        subject systemUnderTest : Agentique;
        doc /* Expose typed, scoped tools to one Assistant. Present proposed semantic changes and scenario changes for Human Operator approval before commit or execution. */
    }

    requirement <'AGQ-SEC01'> sec01 {
        subject systemUnderTest : Agentique;
        doc /* Run simulations without network, shell, build, deployment, or live-agent effects. Treat imported models and Assistant output as untrusted inputs. */
    }

    requirement <'AGQ-NFR01'> nfr01 {
        subject systemUnderTest : Agentique;
        doc /* Persist accepted revisions and acknowledged run checkpoints durably. On restart, preserve the last committed checkpoint and mark an unfinished active run interrupted. */
    }

    requirement <'AGQ-NFR02'> nfr02 {
        subject systemUnderTest : Agentique;
        doc /* Enforce declared input, memory, trace, and step limits and expose progress/cancellation. Use bounded background work for parsing and execution. */
    }

    requirement <'AGQ-EXT01'> ext01 {
        subject systemUnderTest : Agentique;
        doc /* Keep the Rust modelling and simulation libraries independent of HTTP, storage, UI, and AI-provider implementations. Isolate adapters behind typed contracts. */
    }

    requirement <'AGQ-TRACE01'> trace01 {
        subject systemUnderTest : Agentique;
        doc /* Link Agentique model requirements and components to implementation revisions and verification evidence. A trace link alone shall not constitute satisfaction. */
    }

}
