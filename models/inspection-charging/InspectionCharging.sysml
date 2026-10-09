package InspectionCharging {
    doc /* An inspection drone and its charging station, after the Operator's
         * guide "SysML v2: From Syntax to Systems" (its Aster example), as a
         * second system for Stage 13 (C-55).
         *
         * Engineering questions. (1) Does a configured drone stay within its
         * launch-mass limit, with the power bus it shares between roles counted
         * once? (2) Does a station energise its contacts only after the
         * drone's charge permit is accepted, and does a lost answer time out,
         * retry within its budget and end without energising?
         *
         * Boundary and assumptions. Inside: the drone's mass contributions by
         * assembly, the station's charger and its authorisation service, and
         * the charging interface between drone and station. Outside: flight
         * dynamics, electrical protection, current and thermal behaviour,
         * weather, regulation. Masses are declared design values in grams,
         * energy in joules, power in watts and times in milliseconds of
         * logical time: plain numbers, since the ISQ and SI libraries are not
         * loaded (deviation 14), so no unit is checked or converted. Behaviour
         * is discrete and runs in logical time; nothing here verifies a
         * physical drone or charger. The values are illustrative, not design
         * guidance. */

    private import ScalarValues::*;

    // Reusable vocabulary: what every assembly that is counted has.

    part def MassedComponent {
        doc /* An assembly whose mass (g) is counted once in a budget. */
        attribute mass : Real;
    }

    part def Battery :> MassedComponent {
        doc /* Installed energy storage; usableEnergy in J. */
        attribute usableEnergy : Real;
    }

    part def PowerBus :> MassedComponent {
        doc /* The power distribution every powered assembly draws from. */
    }

    part def Camera :> MassedComponent;

    part def Controller :> MassedComponent {
        doc /* A controller assembly, without the bus it draws power from:
             * supply refers to that bus, it does not contain a second one. */
        ref part supply : PowerBus;
    }

    part def PropulsionSet :> MassedComponent {
        doc /* The installed motors as one assembly: motorCount identical
             * motors of motorMass g each. */
        attribute motorCount : Natural;
        attribute motorMass : Real;
        attribute :>> mass = motorCount * motorMass;
    }

    // Charging: what crosses the boundary between drone and station.

    item def ChargePermit {
        doc /* The drone's request to be charged for a mission. */
        attribute missionId : String;
    }

    item def PermitAnswer {
        doc /* The authorisation service's answer to a permit. */
        attribute accepted : Boolean;
    }

    item def ChargeStatus {
        doc /* What the charger tells the drone: whether the contacts are
             * energised, and why not. */
        attribute energized : Boolean;
        attribute reason : String;
    }

    port def ChargingPort {
        doc /* The station's side of the charging contact: a permit comes in,
             * the charge status goes out. The drone uses its conjugate. */
        in item permit : ChargePermit;
        out item status : ChargeStatus;
    }

    port def AuthorizationPort {
        doc /* The authorisation service's side: permits in, answers out. */
        in item request : ChargePermit;
        out item answer : PermitAnswer;
    }

    interface def ChargingLink {
        doc /* The docking contact between a station and a drone. */
        end port station : ChargingPort;
        end port drone : ~ChargingPort;
    }

    part def Drone :> MassedComponent {
        doc /* An inspection drone. Its mass is the sum of its assemblies and
             * its payload; the flight computer's supply is the drone's own bus,
             * referred to, so the bus is counted once. */
        port charging : ~ChargingPort;
        part chassis : MassedComponent;
        part battery : Battery;
        part camera : Camera;
        part bus : PowerBus;
        part flightComputer : Controller {
            ref part :>> supply = bus;
        }
        part propulsion : PropulsionSet;
        attribute payloadMass : Real;
        attribute :>> mass = chassis.mass + battery.mass + camera.mass + bus.mass + flightComputer.mass + propulsion.mass + payloadMass;
    }

    part def AuthorizationService {
        doc /* Decides whether a permit may be charged against. Its decisions
             * are outside this model; scenarios stand it in. */
        port auth : AuthorizationPort;
    }

    part def Charger {
        doc /* Energises the contacts only after the permit was accepted; asks
             * again when no answer comes within timeoutMs, at most maxAttempts
             * times in all, then ends without energising. */
        port plug : ChargingPort;
        port authority : ~AuthorizationPort;
        attribute timeoutMs : Natural = 500;
        attribute maxAttempts : Natural = 2;
        attribute attempts : Natural = 0;
        attribute energized : Boolean = false;
        item pending : ChargePermit[0..1];
        exhibit state charging {
            entry;
            then idle;
            state idle;
            state authorizing;
            state supplying;
            transition request first idle accept p : ChargePermit via plug do action {
                assign pending := p;
                then assign attempts := 1;
                then send p via authority;
            } then authorizing;
            transition accepted first authorizing accept a : PermitAnswer via authority if a.accepted do action {
                assign energized := true;
                then send new ChargeStatus(energized = true, reason = "") via plug;
            } then supplying;
            transition refused first authorizing accept a : PermitAnswer via authority if not a.accepted do send new ChargeStatus(energized = false, reason = "the permit was refused") via plug then idle;
            transition retry first authorizing accept after timeoutMs if attempts < maxAttempts do action {
                assign attempts := attempts + 1;
                then send pending via authority;
            } then authorizing;
            transition givesUp first authorizing accept after timeoutMs if attempts >= maxAttempts do send new ChargeStatus(energized = false, reason = "no answer from the authorisation service") via plug then idle;
        }
    }

    part def ChargingStation {
        doc /* A station: a charger and the authorisation service it asks. The
             * drone being charged is outside the station. */
        port plug : ChargingPort;
        part charger : Charger;
        part authority : AuthorizationService;
        connect plug to charger.plug;
        connect charger.authority to authority.auth;
    }

    part def FieldStation :> ChargingStation {
        doc /* A station on a remote site, with a slow link to the
             * authorisation service: the same charger, configured to wait
             * longer and ask once more. */
        part :>> charger {
            attribute :>> timeoutMs = 2000;
            attribute :>> maxAttempts = 3;
        }
    }

    part def EagerCharger :> Charger {
        doc /* A plausible but wrong design kept to show what the scenarios
             * catch: it energises the contacts as soon as a permit arrives,
             * and asks the authorisation service afterwards. */
        exhibit state :>> charging {
            entry;
            then idle;
            state idle;
            state authorizing;
            state supplying;
            transition request first idle accept p : ChargePermit via plug do action {
                assign energized := true;
                then assign pending := p;
                then assign attempts := 1;
                then send p via authority;
            } then authorizing;
            transition accepted first authorizing accept a : PermitAnswer via authority if a.accepted do send new ChargeStatus(energized = true, reason = "") via plug then supplying;
            transition refused first authorizing accept a : PermitAnswer via authority if not a.accepted do send new ChargeStatus(energized = false, reason = "the permit was refused") via plug then idle;
        }
    }

    part def QuickStation :> ChargingStation {
        doc /* A station built with the eager charger. */
        part :>> charger : EagerCharger;
    }

    // Requirements: reusable rules, applied with their limits.

    requirement def LaunchMassLimit {
        doc /* The drone's launch mass, its assemblies and payload counted once
             * each, shall not exceed the limit. */
        subject aircraft : Drone;
        attribute limit : Real;
        assume constraint positiveLimit { limit > 0 }
        require constraint withinLimit { aircraft.mass <= limit }
    }

    requirement def BoundedWait {
        doc /* A drone waits for a charging decision at most maxWaitMs. */
        subject charger : Charger;
        attribute maxWaitMs : Natural;
        require constraint { charger.timeoutMs * charger.maxAttempts <= maxWaitMs }
    }

    requirement def EnergizeOnlyWhenAuthorized {
        doc /* The contacts are energised only after the authorisation service
             * accepted the drone's permit; a refusal or a lost answer never
             * energises them. */
        subject station : ChargingStation;
    }

    // Configurations: one drone definition, several contextual usages.

    part surveyUnit : Drone {
        part :>> chassis { attribute :>> mass = 1800; }
        part :>> battery {
            attribute :>> mass = 1200;
            attribute :>> usableEnergy = 720000;
        }
        part :>> camera { attribute :>> mass = 250; }
        part :>> bus { attribute :>> mass = 150; }
        part :>> flightComputer { attribute :>> mass = 500; }
        part :>> propulsion {
            attribute :>> motorCount = 4;
            attribute :>> motorMass = 200;
        }
        attribute :>> payloadMass = 1200;
    }

    part heavyUnit : Drone {
        doc /* The same drone after an avionics upgrade, carrying a larger
             * payload. */
        part :>> chassis { attribute :>> mass = 1800; }
        part :>> battery {
            attribute :>> mass = 1200;
            attribute :>> usableEnergy = 720000;
        }
        part :>> camera { attribute :>> mass = 250; }
        part :>> bus { attribute :>> mass = 150; }
        part :>> flightComputer { attribute :>> mass = 650; }
        part :>> propulsion {
            attribute :>> motorCount = 4;
            attribute :>> motorMass = 200;
        }
        attribute :>> payloadMass = 2300;
    }

    part homeStation : ChargingStation;

    part fieldStation : FieldStation;

    requirement launchMass : LaunchMassLimit {
        attribute :>> limit = 7000;
    }

    requirement ferryMass : LaunchMassLimit {
        doc /* Ferry flights without inspection may carry more. */
        attribute :>> limit = 8000;
    }

    requirement shortWait : BoundedWait {
        attribute :>> maxWaitMs = 6000;
    }

    requirement authorizedCharging : EnergizeOnlyWhenAuthorized;

    satisfy launchMass by surveyUnit;
    satisfy launchMass by heavyUnit;
    satisfy ferryMass by heavyUnit;
    satisfy shortWait by homeStation.charger;
    satisfy shortWait by fieldStation.charger;
    satisfy authorizedCharging by homeStation;

    // Scenarios: the bounded behavioural question, success and failure.

    verification def ChargingAuthorized {
        doc /* The permit is accepted: the contacts are energised. */
        subject station : ChargingStation;
        objective { verify authorizedCharging; }
        part accepts : Scenarios::StandIn {
            :>> target = station.authority;
            :>> outcome = Scenarios::Outcome::answer;
            :>> latencyMs = 120;
            :>> output = new PermitAnswer(accepted = true);
        }
        send new ChargePermit(missionId = "bridge-17") via station.plug;
        then accept status : ChargeStatus via station.plug;
        then assert constraint energizedAfterAcceptance { status.energized and station.charger.energized }
    }

    verification def ChargingRefused {
        doc /* The permit is refused: nothing is energised. */
        subject station : ChargingStation;
        objective { verify authorizedCharging; }
        part refuses : Scenarios::StandIn {
            :>> target = station.authority;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new PermitAnswer(accepted = false);
        }
        send new ChargePermit(missionId = "bridge-17") via station.plug;
        then accept status : ChargeStatus via station.plug;
        then assert constraint notEnergized { not status.energized and not station.charger.energized }
    }

    verification def AnswerLostOnce {
        doc /* The first answer is lost: the charger asks again after its
             * timeout and energises on the second answer. */
        subject station : ChargingStation;
        objective { verify authorizedCharging; }
        part lost : Scenarios::StandIn {
            :>> target = station.authority;
            :>> call = 1;
            :>> outcome = Scenarios::Outcome::timeout;
        }
        part answers : Scenarios::StandIn {
            :>> target = station.authority;
            :>> call = 2;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new PermitAnswer(accepted = true);
        }
        send new ChargePermit(missionId = "bridge-17") via station.plug;
        then accept status : ChargeStatus via station.plug;
        then assert constraint recoveredAfterRetry { status.energized and station.charger.attempts == 2 }
    }

    verification def AnswersLostTwiceAtHome {
        doc /* Two answers are lost: a home station gives up without
             * energising. */
        subject station : ChargingStation;
        objective { verify authorizedCharging; }
        part lost : Scenarios::StandIn {
            :>> target = station.authority;
            :>> outcome = Scenarios::Outcome::timeout;
        }
        send new ChargePermit(missionId = "bridge-17") via station.plug;
        then accept status : ChargeStatus via station.plug;
        then assert constraint gaveUpUnenergized { not status.energized and station.charger.attempts == 2 }
    }

    verification def AnswersLostTwiceInTheField {
        doc /* The same losses at a field station: its third attempt is
             * answered, so it energises. The charger definition is the same;
             * only the field station's configuration differs. */
        subject station : FieldStation;
        objective { verify authorizedCharging; }
        part lostFirst : Scenarios::StandIn {
            :>> target = station.authority;
            :>> call = 1;
            :>> outcome = Scenarios::Outcome::timeout;
        }
        part lostSecond : Scenarios::StandIn {
            :>> target = station.authority;
            :>> call = 2;
            :>> outcome = Scenarios::Outcome::timeout;
        }
        part answers : Scenarios::StandIn {
            :>> target = station.authority;
            :>> call = 3;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new PermitAnswer(accepted = true);
        }
        send new ChargePermit(missionId = "bridge-17") via station.plug;
        then accept status : ChargeStatus via station.plug;
        then assert constraint energizedOnThirdAttempt { status.energized and station.charger.attempts == 3 }
    }

    verification def RefusedAtAQuickStation {
        doc /* The refused permit at a station with the eager charger: the
             * check fails, because the contacts were energised before the
             * answer came. The scenario finds the design flaw. */
        subject station : QuickStation;
        objective { verify authorizedCharging; }
        part refuses : Scenarios::StandIn {
            :>> target = station.authority;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new PermitAnswer(accepted = false);
        }
        send new ChargePermit(missionId = "bridge-17") via station.plug;
        then accept status : ChargeStatus via station.plug;
        then assert constraint notEnergized { not status.energized and not station.charger.energized }
    }
}
