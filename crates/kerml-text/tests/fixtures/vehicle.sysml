package Mobility {
    attribute def Quantity;
    item def Fuel;

    port def FuelPort {
        out item fuel : Fuel;
    }
    port def MeteredFuelPort :> FuelPort {
        attribute flowRate : Quantity;
    }

    part def Engine {
        attribute power : Quantity;
        port fuelIn : FuelPort;
    }
    part def TurboEngine :> Engine {
        attribute boost : Quantity;
    }
    part def FuelTank {
        item contents : Fuel;
        port fuelOut : FuelPort;
    }
    connection def FuelLine {
        end port leftEnd : FuelPort;
        end port rightEnd : FuelPort;
    }

    action def Drive {
        in item requestedFuel : Fuel;
        out item deliveredFuel : Fuel;
    }
    state def Ready {
        entry action entering;
        do action running : Drive;
        exit action leaving;
    }

    part def Vehicle {
        part engine : Engine;
        part tank : FuelTank;
        attribute mass : Quantity;
        port supply : MeteredFuelPort;
        port returnFlow : FuelPort;
        connection fuelLine : FuelLine connect supply to returnFlow;
        action driving : Drive;
        state operatingState : Ready;
    }
    part def SportsCar :> Vehicle {
        part performanceEngine : TurboEngine :>> Vehicle::engine;
    }

    requirement def RoadReady {
        subject checkedVehicle : Vehicle;
        require constraint inspectionPassed { true }
    }
}
