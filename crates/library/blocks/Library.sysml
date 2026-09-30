package Library {
    doc /* Agentique's built-in building blocks for software systems: a small set
         * of neutral, technology-independent concepts to compose instead of
         * modelling common structure from scratch. Using a block copies the
         * definitions it needs into the project's own Library package, so a
         * project never depends on this text. */

    package Messages {
        doc /* The items that flow between components. */
        item def Request {
            doc /* A call to a component: what is asked for, with its input. */
        }
        item def Response {
            doc /* The answer to a request: a result or an error. */
        }
        item def Message {
            doc /* Data sent to a component without waiting for an answer. */
        }
        item def Event :> Message {
            doc /* A message that records something that has happened. */
        }
    }

    package Interfaces {
        doc /* How components meet: requests with answers, or one-way messages. */
        port def RequestPort {
            doc /* Where a component serves requests: requests come in and responses
                 * go out. Its conjugate, ~RequestPort, is where a component makes
                 * requests. */
            in item request : Messages::Request;
            out item response : Messages::Response;
        }
        port def MessagePort {
            doc /* Where a component receives messages. Its conjugate, ~MessagePort,
                 * is where a component sends them. */
            in item payload : Messages::Message;
        }
        interface def RequestResponse {
            doc /* A caller connected to the component that serves its requests. */
            end port client : ~RequestPort;
            end port server : RequestPort;
        }
        interface def MessageChannel {
            doc /* A sender connected to the component that receives its messages. */
            end port sender : ~MessagePort;
            end port receiver : MessagePort;
        }
    }

    package Services {
        doc /* Components that serve requests, and the components that stand in
             * front of them. */
        part def Service {
            doc /* A component that serves requests through api. */
            port api : Interfaces::RequestPort;
        }
        part def Gateway :> Service {
            doc /* The single entry point of a system: it serves requests from
                 * outside through api and forwards each one to the backend that
                 * handles it. */
            port backend : ~Interfaces::RequestPort;
        }
        part def Proxy {
            doc /* A component that stands in front of another: it takes requests on
                 * inbound and passes them on through outbound. */
            port inbound : Interfaces::RequestPort;
            port outbound : ~Interfaces::RequestPort;
        }
        part def RateLimiter :> Proxy {
            doc /* A proxy that refuses a client's requests beyond limitPerMinute in
                 * any minute. */
            attribute limitPerMinute : ScalarValues::Positive = 600;
        }
        part def Authenticator :> Proxy {
            doc /* A proxy that passes on only requests whose credentials identities
                 * accepts, and answers every other request with an error. */
            port identities : ~Interfaces::RequestPort;
        }
        part def RateLimitedApi :> Gateway {
            doc /* A public API that limits how often each client may call it:
                 * requests pass through a rate limiter before they reach backend. */
            part limiter : RateLimiter;
            connection connect api to limiter.inbound;
            connection connect limiter.outbound to backend;
            requirement fairUse : FairUse;
            satisfy fairUse by limiter;
        }
        requirement def FairUse {
            doc /* No client is served more requests per minute than the limit
                 * allows. */
            subject limiter : RateLimiter;
        }
    }

    package Storage {
        doc /* Components that keep data. */
        part def Store {
            doc /* Keeps data durably and serves reads and writes through access. */
            port access : Interfaces::RequestPort;
        }
        part def KeyValueStore :> Store {
            doc /* A store that reads and writes each value by its key. */
        }
        part def Cache {
            doc /* Answers repeated reads from memory: a read it cannot answer goes
                 * on to backend, and the answer is kept for ttlSeconds. */
            port access : Interfaces::RequestPort;
            port backend : ~Interfaces::RequestPort;
            attribute ttlSeconds : ScalarValues::Positive = 300;
            attribute maxEntries : ScalarValues::Positive [0..1];
        }
        part def CachedStore :> Store {
            doc /* A store with a cache in front of it: reads are answered from the
                 * cache when they can be, and everything else reaches the store. */
            part cache : Cache;
            part store : KeyValueStore;
            connection connect access to cache.access;
            interface storage : Interfaces::RequestResponse connect cache.backend to store.access;
            requirement freshReads : FreshReads;
            satisfy freshReads by cache;
        }
        requirement def FreshReads {
            doc /* A read never returns a value older than the cache's ttlSeconds. */
            subject cache : Cache;
        }
    }

    package Messaging {
        doc /* Components that pass messages on without making the sender wait. */
        part def Queue {
            doc /* Keeps messages in order until a consumer takes them, so producers
                 * and consumers work at their own pace. A message whose processing
                 * fails is delivered again, at most maxDeliveries times. */
            port producers : Interfaces::MessagePort;
            port consumers : ~Interfaces::MessagePort;
            attribute maxDeliveries : ScalarValues::Positive = 3;
        }
        part def Topic {
            doc /* Publish and subscribe: every message published is delivered to
                 * each subscriber. */
            port publishers : Interfaces::MessagePort;
            port subscribers : ~Interfaces::MessagePort;
        }
        part def Worker {
            doc /* Processes the messages that arrive at jobs in the background, at
                 * most concurrency at a time. */
            port jobs : Interfaces::MessagePort;
            attribute concurrency : ScalarValues::Positive = 1;
        }
        part def Scheduler {
            doc /* Sends a message every intervalSeconds, to start work that no
                 * request asks for. */
            port ticks : ~Interfaces::MessagePort;
            attribute intervalSeconds : ScalarValues::Positive = 60;
        }
        part def AsyncWorker {
            doc /* Accepts jobs without making the sender wait: a queue in front of a
                 * worker, and a job that fails is delivered again. */
            port jobs : Interfaces::MessagePort;
            part queue : Queue;
            part worker : Worker;
            connection connect jobs to queue.producers;
            interface delivery : Interfaces::MessageChannel connect queue.consumers to worker.jobs;
            requirement everyJob : JobsProcessed;
            satisfy everyJob by queue;
        }
        part def EventProcessing {
            doc /* An event-driven processing path: events published to events are
                 * delivered by a topic to each processor, and every processor
                 * works through them behind its own queue, so a slow one never
                 * holds up the others. */
            port events : Interfaces::MessagePort;
            part topic : Topic;
            part processors : AsyncWorker [1..*];
            connection connect events to topic.publishers;
            interface fanOut : Interfaces::MessageChannel connect topic.subscribers to processors.jobs;
        }
        requirement def JobsProcessed {
            doc /* Every accepted job is processed, or kept for inspection when its
                 * last delivery fails. */
            subject queue : Queue;
        }
    }

    package Resilience {
        doc /* Structures that keep a system answering when a part of it fails. */
        part def Retry :> Services::Proxy {
            doc /* A proxy that tries a failed request again, up to maxAttempts
                 * times, waiting backoffMs longer each time. */
            attribute maxAttempts : ScalarValues::Positive = 3;
            attribute backoffMs : ScalarValues::Natural = 200;
        }
        part def CircuitBreaker :> Services::Proxy {
            doc /* A proxy that stops calling outbound after failureThreshold
                 * failures in a row, answers at once with an error instead, and
                 * tries again after resetSeconds. */
            attribute failureThreshold : ScalarValues::Positive = 5;
            attribute resetSeconds : ScalarValues::Positive = 30;
        }
        part def Fallback :> Services::Proxy {
            doc /* A proxy that asks secondary instead whenever outbound fails. */
            port secondary : ~Interfaces::RequestPort;
        }
        item def Job {
            doc /* Work to do, by id. */
            attribute id : ScalarValues::String;
        }
        item def JobResult {
            doc /* How a job ended: done or failed, after how many attempts. */
            attribute id : ScalarValues::String;
            attribute ok : ScalarValues::Boolean;
            attribute attempts : ScalarValues::Natural;
        }
        item def Attempt {
            doc /* One try at a job. */
            attribute id : ScalarValues::String;
            attribute attempt : ScalarValues::Positive;
        }
        item def AttemptResult {
            doc /* Whether one try worked. */
            attribute id : ScalarValues::String;
            attribute ok : ScalarValues::Boolean;
        }
        port def JobPort {
            doc /* Where jobs come in and their results go out. */
            in item job : Job;
            out item result : JobResult;
        }
        port def AttemptPort {
            doc /* Where tries go out to what does the work, and their results come
                 * back. */
            out item attempt : Attempt;
            in item result : AttemptResult;
        }
        part def RetryingWorker {
            doc /* Takes each job at jobs and tries it through target; a failed try
                 * is tried again, up to maxAttempts times, waiting backoffMs longer
                 * each time. The result says whether the job was done and after
                 * how many attempts. Its behaviour runs in scenarios. */
            port jobs : JobPort;
            port target : AttemptPort;
            attribute maxAttempts : ScalarValues::Positive = 3;
            attribute backoffMs : ScalarValues::Natural = 200;
            attribute attempts : ScalarValues::Natural = 0;
            attribute current : ScalarValues::String = "";
            exhibit state working {
                entry;
                then idle;
                state idle;
                state trying;
                state waiting;
                transition first idle accept job : Job via jobs do action {
                    assign current := job.id;
                    then assign attempts := 1;
                    then send new Attempt(id = job.id, attempt = 1) via target;
                } then trying;
                transition first trying accept r : AttemptResult via target if r.ok do send new JobResult(id = current, ok = true, attempts = attempts) via jobs then idle;
                transition first trying accept r : AttemptResult via target if not r.ok and attempts < maxAttempts then waiting;
                transition first trying accept r : AttemptResult via target if not r.ok and attempts >= maxAttempts do send new JobResult(id = current, ok = false, attempts = attempts) via jobs then idle;
                transition first waiting accept after backoffMs * attempts do action {
                    assign attempts := attempts + 1;
                    then send new Attempt(id = current, attempt = attempts) via target;
                } then trying;
            }
        }
        verification def RetryThenDone {
            doc /* Two tries fail and the third works: the job is done after three
                 * attempts. */
            subject worker : RetryingWorker;
            send new Job(id = "j1") via worker.jobs;
            then accept firstTry : Attempt via worker.target;
            then send new AttemptResult(id = "j1", ok = false) via worker.target;
            then accept secondTry : Attempt via worker.target;
            then send new AttemptResult(id = "j1", ok = false) via worker.target;
            then accept thirdTry : Attempt via worker.target;
            then send new AttemptResult(id = "j1", ok = true) via worker.target;
            then accept result : JobResult via worker.jobs;
            then assert constraint doneAfterThree {
                result.ok and result.attempts == 3 and thirdTry.attempt == 3
            }
        }
        verification def GiveUp {
            doc /* Every try fails: after maxAttempts the job is reported failed. */
            subject worker : RetryingWorker;
            send new Job(id = "j2") via worker.jobs;
            then accept firstTry : Attempt via worker.target;
            then send new AttemptResult(id = "j2", ok = false) via worker.target;
            then accept secondTry : Attempt via worker.target;
            then send new AttemptResult(id = "j2", ok = false) via worker.target;
            then accept thirdTry : Attempt via worker.target;
            then send new AttemptResult(id = "j2", ok = false) via worker.target;
            then accept result : JobResult via worker.jobs;
            then assert constraint failedAfterThree {
                not result.ok and result.attempts == 3
            }
        }
    }

    package Moderation {
        doc /* Checking what people submit with an AI agent, and a deterministic
             * path for when the agent cannot answer well enough. */
        enum def ModerationDecision {
            doc /* What may happen to a submission: published, reviewed by a person,
                 * or rejected. */
            enum publish;
            enum review;
            enum reject;
        }
        item def Submission {
            doc /* Something submitted to be checked: its text. */
            attribute text : ScalarValues::String;
        }
        item def ModerationVerdict :> Agents::AgentOutput {
            doc /* What the check decided, and why. */
            attribute decision : ModerationDecision;
            attribute reason : ScalarValues::String [0..1];
        }
        port def ModerationPort {
            doc /* Where submissions come in and verdicts go out. */
            in item submission : Submission;
            out item verdict : ModerationVerdict;
        }
        abstract part def Moderator {
            doc /* The moderation contract, shared by the agent and its fallback. */
            port check : ModerationPort;
        }
        part def ModerationAgent :> Moderator, Agents::Agent {
            doc /* Decides whether a submission may be published. Answer publish for
                 * ordinary content, review when unsure, and reject for clear
                 * abuse. Give a confidence between 0 and 1 and a short reason. */
            :>> mode = Agents::AgentMode::fast;
            :>> minConfidence = 0.8;
            :>> maxLatencyMs = 1000;
            part : HoldForReview :>> fallback;
        }
        part def HoldForReview :> Moderator {
            doc /* The deterministic path: every submission waits for a person. */
            exhibit state holding {
                entry;
                then ready;
                state ready;
                transition first ready accept s : Submission via check do send new ModerationVerdict(decision = ModerationDecision::review, confidence = 1.0, reason = "held for a person") via check then ready;
            }
        }
        part def ModeratedInbox {
            doc /* Where submissions arrive, each checked by the moderation agent. */
            port submissions : ModerationPort;
            part moderator : ModerationAgent;
            connection connect submissions to moderator.check;
        }
        verification def ConfidentPublish {
            doc /* The agent is sure: its verdict stands. */
            subject inbox : ModeratedInbox;
            part sure : Scenarios::StandIn {
                :>> target = inbox.moderator;
                :>> outcome = Scenarios::Outcome::answer;
                :>> latencyMs = 200;
                :>> output = new ModerationVerdict(decision = ModerationDecision::publish, confidence = 0.95);
            }
            send new Submission(text = "hello") via inbox.submissions;
            then accept verdict : ModerationVerdict via inbox.submissions;
            then assert constraint published {
                verdict.decision == ModerationDecision::publish
            }
        }
        verification def AgentFailsToReview {
            doc /* The agent does not answer in time: a person reviews it. */
            subject inbox : ModeratedInbox;
            part slow : Scenarios::StandIn {
                :>> target = inbox.moderator;
                :>> outcome = Scenarios::Outcome::timeout;
            }
            send new Submission(text = "hello") via inbox.submissions;
            then accept verdict : ModerationVerdict via inbox.submissions;
            then assert constraint heldForReview {
                verdict.decision == ModerationDecision::review
            }
        }
    }
}
