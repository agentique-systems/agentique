package Notifications {
    doc /* A notification service whose dispatcher retries a failing SMS
         * gateway: the second, smaller example of Scenario I (C-50).
         */

    private import ScalarValues::*;

    enum def DeliveryStatus {
        enum delivered;
        enum failed;
    }

    item def Notification {
        attribute id : String;
        attribute text : String;
    }

    item def SendRequest {
        attribute id : String;
        attribute attempt : Positive;
    }

    item def SendResult {
        attribute id : String;
        attribute ok : Boolean;
    }

    item def Receipt {
        attribute id : String;
        attribute status : DeliveryStatus;
        attribute attempts : Natural;
    }

    port def NotificationPort {
        in item notification : Notification;
        out item receipt : Receipt;
    }

    port def GatewayPort {
        out item request : SendRequest;
        in item result : SendResult;
    }

    part def Dispatcher {
        doc /* Sends each notification to the gateway, and retries a failed send
             * up to maxAttempts times, waiting backoffMs longer each time.
             */
        port inbox : NotificationPort;
        port gateway : GatewayPort;
        attribute maxAttempts : Positive = 3;
        attribute backoffMs : Natural = 200;
        attribute attempts : Natural = 0;
        attribute current : String = "";
        exhibit state dispatching {
            entry;
            then idle;
            state idle;
            state sending;
            state waiting;
            transition first idle accept n : Notification via inbox do action {
                assign current := n.id;
                then assign attempts := 1;
                then send new SendRequest(id = n.id, attempt = 1) via gateway;
            } then sending;
            transition first sending accept r : SendResult via gateway if r.ok do send new Receipt(id = current, status = DeliveryStatus::delivered, attempts = attempts) via inbox then idle;
            transition first sending accept r : SendResult via gateway if not r.ok and attempts < maxAttempts then waiting;
            transition first sending accept r : SendResult via gateway if not r.ok and attempts >= maxAttempts do send new Receipt(id = current, status = DeliveryStatus::failed, attempts = attempts) via inbox then idle;
            transition first waiting accept after backoffMs * attempts do action {
                assign attempts := attempts + 1;
                then send new SendRequest(id = current, attempt = attempts) via gateway;
            } then sending;
        }
    }

    part def SmsGateway {
        doc /* Sends a text message and answers whether it went out. */
        port requests : ~GatewayPort;
    }

    part def NotificationService {
        port notifications : NotificationPort;
        part dispatcher : Dispatcher;
        part gateway : SmsGateway;
        connect notifications to dispatcher.inbox;
        connect dispatcher.gateway to gateway.requests;
    }

    dependency from Dispatcher to SmsGateway {
        doc /* The dispatcher sends through the gateway's contract. */
    }

    requirement def DeliveredOnce {
        doc /* Every accepted notification is delivered at most once, and a
             * notification the gateway keeps refusing ends as failed.
             */
        subject service : NotificationService;
    }

    requirement deliveredOnce : DeliveredOnce;

    verification def RetryThenDeliver {
        doc /* The gateway fails twice, then sends: one receipt, delivered on
             * the third attempt.
             */
        subject service : NotificationService;
        objective {
            verify deliveredOnce;
        }
        part failFirst : Scenarios::StandIn {
            :>> target = service.gateway;
            :>> call = 1;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new SendResult(id = "n1", ok = false);
        }
        part failSecond : Scenarios::StandIn {
            :>> target = service.gateway;
            :>> call = 2;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new SendResult(id = "n1", ok = false);
        }
        part succeedThird : Scenarios::StandIn {
            :>> target = service.gateway;
            :>> call = 3;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new SendResult(id = "n1", ok = true);
        }
        send new Notification(id = "n1", text = "hello") via service.notifications;
        then accept receipt : Receipt via service.notifications;
        then assert constraint deliveredOnThirdAttempt {
            receipt.status == DeliveryStatus::delivered and receipt.attempts == 3
        }
    }

    verification def GiveUpAfterThreeAttempts {
        doc /* The gateway keeps failing: after maxAttempts the notification ends
             * as failed, and the gateway is not called again.
             */
        subject service : NotificationService;
        objective {
            verify deliveredOnce;
        }
        part alwaysFail : Scenarios::StandIn {
            :>> target = service.gateway;
            :>> outcome = Scenarios::Outcome::answer;
            :>> latencyMs = 50;
            :>> output = new SendResult(id = "n2", ok = false);
        }
        send new Notification(id = "n2", text = "hello again") via service.notifications;
        then accept receipt : Receipt via service.notifications;
        then assert constraint endsAsFailed {
            receipt.status == DeliveryStatus::failed and receipt.attempts == 3
        }
        then accept after 5000;
        then assert constraint dispatcherIsIdle {
            service.dispatcher.attempts == 3
        }
    }
}
