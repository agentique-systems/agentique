package Controller {
    private import ScalarValues::*;
    doc /* Independent fixture derived from SysML 2.0 Part 1, 7.18.3 OnOff2/OnOff3.
         Names, states, ordering, guard and payload differ from the Agentique lifecycle. */
    item def Switch;
    item def Report {
        attribute count : Integer;
    }
    state def Mode {
        in attribute enabled : Boolean;
        in attribute threshold : Integer;
        port commands;
        entry;
        then dormant;
        state live;
        state dormant;
        state retired;
        transition wake first dormant accept Switch via commands if enabled then live;
        transition sleep first live accept Switch via commands then dormant;
        transition retire first live accept r : Report via commands if r.count >= threshold then retired;
    }
    part def Device { exhibit state mode : Mode; }
    part firstDevice : Device;
    part secondDevice : Device;
}
