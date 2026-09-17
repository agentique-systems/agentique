package AgentiqueBehaviour {
    doc /* AGQ v0.1 draft: one-request validation lifecycle.
       Scope: a behaviour slice of Agentique, not a model of the complete service.
       Syntax and semantics require independent tool validation at milestone M0.
       The runtime supplies scenario inputs to the input port; it does not run a validator here.
       Accepted and rejected are ordinary stable states, not SysML final states.
    */

    item def CheckRequested;
    item def ValidationPassed;
    item def ValidationFailed;

    state def RequestLifecycle {
        port input;
        entry;
        then idle;

        state idle;
        state checking;
        state accepted;
        state rejected;

        transition beginCheck
            first idle
            accept CheckRequested via input
            then checking;

        transition acceptCheck
            first checking
            accept ValidationPassed via input
            then accepted;

        transition rejectCheck
            first checking
            accept ValidationFailed via input
            then rejected;
    }
}
