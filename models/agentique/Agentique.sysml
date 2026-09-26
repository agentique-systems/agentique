package AgentiqueArchitecture {
    doc /* The logical architecture of Agentique. Each part definition names the crates
         * that implement it; the dependencies at the end say which parts may use which.
         * tools/check_architecture.py compares both with the Cargo crate graph (R-15). */

    part def Crate {
        doc /* A Cargo package. A part usage typed by Crate and named after the package
             * says that the crate implements the enclosing part. */
    }

    // Items exchanged between the parts.

    item def ChangeOperation {
        doc /* A typed change to the model, such as adding a part or setting a value.
             * Taking a lock and undoing a change are operations too. */
    }
    item def ChangeEvent {
        doc /* Notice that an operation changed the System State. */
    }
    item def Query {
        doc /* A read-only request, such as the content of a view. */
    }
    item def QueryResult;
    item def SysMLText {
        doc /* A project's model in SysML v2 textual notation. */
    }
    item def ChatMessage {
        doc /* A message between the Operator and the Assistant. */
    }

    port def SystemStatePort {
        doc /* Typed change operations and queries into the System State, and the
             * change events it emits. */
        in item operation : ChangeOperation;
        in item query : Query;
        out item result : QueryResult;
        out item change : ChangeEvent;
    }
    port def HistoryPort {
        doc /* Saving and loading a project as SysML text. */
        in item saved : SysMLText;
        out item loaded : SysMLText;
    }
    port def AssistantPort {
        doc /* The Operator's messages to the Assistant and its replies. */
        in item request : ChatMessage;
        out item reply : ChatMessage;
    }

    // The parts and the crates that implement them.

    part def LanguageCore {
        doc /* The KerML/SysML subset: element graph, text parse and print, name
             * resolution, typing, specialisation and validation. Locked core (R-16). */
        part 'agq-kernel' : Crate;
        part 'agq-kerml' : Crate;
        part 'agq-sysml' : Crate;
        part 'agq-kerml-syntax' : Crate;
        part 'agq-kerml-text' : Crate;
        part 'agq-kerml-semantics' : Crate;
        part 'agq-sysml-semantics' : Crate;
        part 'agq-standard-libraries' : Crate;
        part 'agq-runtime-publications' : Crate;
        part 'agq-metamodel-gen' : Crate {
            doc /* Generates the KerML/SysML metamodel descriptors. */
        }
    }

    part def SystemState {
        doc /* The live state of one project: typed change operations, locks, undo,
             * change events and queries for views. Its operations are locked core (R-16). */
        port access : SystemStatePort;
        port files : ~HistoryPort;
        part 'agq-modeling-workspace' : Crate;
        part 'agq-modeling-service' : Crate;
        part 'agq-modeling-view' : Crate {
            doc /* Queries for views. */
        }
    }

    part def History {
        doc /* Saves and loads projects as SysML text in git, with checkpoints, branches
             * and diffs. The persistence format is locked core (R-16). Until Stage 2
             * replaces it with git, History is the SQLite revision store below. */
        port files : HistoryPort;
        part 'agq-modeling-repository' : Crate;
        part 'agq-modeling-sqlite' : Crate;
    }

    part def Studio {
        doc /* The application shell the Operator works in. */
        part surface {
            doc /* Spatial rendering of the model and direct manipulation. */
        }
        part panels {
            doc /* Inspector, requirements, history, simulation and links. */
        }
        part conversation {
            doc /* Chat with the Assistant. */
            port chat : ~AssistantPort;
        }
        port stateAccess : ~SystemStatePort;
        part 'agq-studio-native' : Crate;
        part 'agq-studio-platform' : Crate;
        part 'agq-studio-scene' : Crate;
    }

    part def Assistant {
        doc /* The Claude API client: tool-use loop, skills, and tools that call System
             * State operations. An Orchestrator comes later. */
        port chat : AssistantPort;
        port stateAccess : ~SystemStatePort;
        part 'agq-modeling-agent' : Crate;
    }

    part def Simulation {
        doc /* Stage 4: scenarios run over the architecture. No crate yet. */
    }

    part def ImplementationLinks {
        doc /* Stage 5: links from model elements to code and tests, and drift checks.
             * No crate yet. */
        port stateAccess : ~SystemStatePort;
    }

    part def Agentique {
        doc /* The product: its parts and how they connect. */
        part languageCore : LanguageCore;
        part systemState : SystemState;
        part history : History;
        part studio : Studio;
        part assistant : Assistant;
        part simulation : Simulation;
        part implementationLinks : ImplementationLinks;

        connect studio.stateAccess to systemState.access;
        connect assistant.stateAccess to systemState.access;
        connect implementationLinks.stateAccess to systemState.access;
        connect systemState.files to history.files;
        connect studio.conversation.chat to assistant.chat;
    }

    // Allowed dependencies, client to supplier. A crate may use crates of its own
    // part and of every part reachable through these dependencies.

    dependency from Studio to SystemState;
    dependency from Assistant to SystemState;
    dependency from SystemState to LanguageCore;
    dependency from SystemState to History;
    dependency from Simulation to LanguageCore;
    dependency from ImplementationLinks to SystemState;

    dependency from History to SystemState {
        doc /* Temporary: agq-modeling-repository uses revision and project types from
             * agq-modeling-workspace and, through it, LanguageCore crates. Stage 2
             * replaces it with git-backed History, which removes this dependency. */
    }
}
