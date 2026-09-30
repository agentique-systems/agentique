package UrlShortener {
    doc /* A URL shortener whose new links are screened by an AI agent, with a
         * deterministic recovery path and a rule for links that need review:
         * Scenario I (ROADMAP §2.10, C-50). The acceptance behaviour was fixed
         * before any implementation: a confident allow activates a link;
         * review, low confidence or a failed screening holds it for review;
         * block stores nothing; only an active link redirects; a reviewer's
         * approval activates a held link.
         */

    private import ScalarValues::*;

    enum def LinkStatus {
        enum active;
        enum held;
        enum blocked;
    }

    enum def Decision {
        enum allow;
        enum review;
        enum block;
    }

    enum def ResolveOutcome {
        enum redirect;
        enum held;
        enum unknown;
    }

    item def ShortenRequest {
        attribute longUrl : String;
        attribute host : String;
    }

    item def ShortLink {
        attribute code : String;
        attribute longUrl : String;
        attribute status : LinkStatus;
    }

    item def ResolveRequest {
        attribute code : String;
    }

    item def Resolution {
        attribute outcome : ResolveOutcome;
        attribute location : String;
    }

    item def ReviewDecision {
        attribute code : String;
        attribute approve : Boolean;
    }

    item def LinkQuery {
        attribute code : String;
    }

    item def StatusChange {
        attribute code : String;
        attribute status : LinkStatus;
    }

    item def LinkRecord {
        attribute code : String;
        item link : ShortLink[0..1];
    }

    item def LinkCandidate {
        attribute longUrl : String;
        attribute host : String;
    }

    item def Verdict :> Agents::AgentOutput {
        attribute decision : Decision;
        attribute reason : String[0..1];
    }

    port def ShortenPort {
        in item request : ShortenRequest;
        out item link : ShortLink;
    }

    port def ResolvePort {
        in item request : ResolveRequest;
        out item resolution : Resolution;
    }

    port def ReviewPort {
        in item decision : ReviewDecision;
        out item link : ShortLink;
    }

    port def LinkStorePort {
        in item save : ShortLink;
        in item query : LinkQuery;
        in item change : StatusChange;
        out item record : LinkRecord;
    }

    port def ScreeningPort {
        in item candidate : LinkCandidate;
        out item verdict : Verdict;
    }

    abstract part def Screening {
        doc /* The screening contract, shared by the agent and its fallback: a
             * candidate link comes in, a verdict goes out.
             */
        port check : ScreeningPort;
    }

    part def LinkScreening :> Screening, Agents::Agent {
        doc /* Decides whether a new short link may go live. Answer allow for an
             * ordinary link, review when unsure or when the site looks risky,
             * and block for known abuse (malware, phishing). Give a confidence
             * between 0 and 1 and a short reason.
             */
        :>> mode = Agents::AgentMode::fast;
        :>> model = "deepseek-flash";
        :>> minConfidence = 0.8;
        :>> maxLatencyMs = 500;
        part : BlocklistScreening :>> fallback;
    }

    part def BlocklistScreening :> Screening {
        doc /* The deterministic recovery path: a host on the blocklist is
             * blocked; anything else is held for review.
             */
        attribute blockedHost : String = "malware.example";
        exhibit state deciding {
            entry;
            then ready;
            state ready;
            transition first ready accept c : LinkCandidate via check if c.host == blockedHost do send new Verdict(decision = Decision::block, confidence = 1.0, reason = "the host is on the blocklist") via check then ready;
            transition first ready accept c : LinkCandidate via check if c.host != blockedHost do send new Verdict(decision = Decision::review, confidence = 1.0, reason = "screening was not available") via check then ready;
        }
    }

    part def LinkApi {
        doc /* The public API: shortens, resolves and reviews links. A new link
             * is screened first; its status follows the verdict.
             */
        port shorten : ShortenPort;
        port resolve : ResolvePort;
        port review : ReviewPort;
        port screening : ~ScreeningPort;
        port storage : ~LinkStorePort;
        attribute nextCode : String = "abc123";
        attribute pendingUrl : String = "";
        exhibit state serving {
            entry;
            then idle;
            state idle;
            state awaitingVerdict;
            state resolving;
            state reviewing;
            transition first idle accept req : ShortenRequest via shorten do action {
                assign pendingUrl := req.longUrl;
                then send new LinkCandidate(longUrl = req.longUrl, host = req.host) via screening;
            } then awaitingVerdict;
            transition first awaitingVerdict accept v : Verdict via screening if v.decision == Decision::allow do action {
                send new ShortLink(code = nextCode, longUrl = pendingUrl, status = LinkStatus::active) via storage;
                then send new ShortLink(code = nextCode, longUrl = pendingUrl, status = LinkStatus::active) via shorten;
            } then idle;
            transition first awaitingVerdict accept v : Verdict via screening if v.decision == Decision::review do action {
                send new ShortLink(code = nextCode, longUrl = pendingUrl, status = LinkStatus::held) via storage;
                then send new ShortLink(code = nextCode, longUrl = pendingUrl, status = LinkStatus::held) via shorten;
            } then idle;
            transition first awaitingVerdict accept v : Verdict via screening if v.decision == Decision::block do send new ShortLink(code = "", longUrl = pendingUrl, status = LinkStatus::blocked) via shorten then idle;
            transition first idle accept q : ResolveRequest via resolve do send new LinkQuery(code = q.code) via storage then resolving;
            transition first resolving accept r : LinkRecord via storage if r.link != null and r.link.status == LinkStatus::active do send new Resolution(outcome = ResolveOutcome::redirect, location = r.link.longUrl) via resolve then idle;
            transition first resolving accept r : LinkRecord via storage if r.link != null and r.link.status != LinkStatus::active do send new Resolution(outcome = ResolveOutcome::held, location = "") via resolve then idle;
            transition first resolving accept r : LinkRecord via storage if r.link == null do send new Resolution(outcome = ResolveOutcome::unknown, location = "") via resolve then idle;
            transition first idle accept d : ReviewDecision via review do send new StatusChange(code = d.code, status = if d.approve ? LinkStatus::active else LinkStatus::blocked) via storage then reviewing;
            transition first reviewing accept r : LinkRecord via storage if r.link != null do send r.link via review then idle;
            transition first reviewing accept r : LinkRecord via storage if r.link == null do send new ShortLink(code = r.code, longUrl = "", status = LinkStatus::blocked) via review then idle;
        }
    }

    part def LinkStore {
        doc /* Keeps a short link and answers queries and status changes for it.
             * The model keeps the most recent link only; each scenario works
             * with one link.
             */
        port links : LinkStorePort;
        item stored : ShortLink[0..1];
        exhibit state keeping {
            entry;
            then ready;
            state ready;
            transition first ready accept link : ShortLink via links do assign stored := link then ready;
            transition first ready accept q : LinkQuery via links do send new LinkRecord(code = q.code, link = if stored != null and stored.code == q.code ? stored else null) via links then ready;
            transition first ready accept c : StatusChange via links if stored != null and stored.code == c.code do action {
                assign stored.status := c.status;
                then send new LinkRecord(code = c.code, link = stored) via links;
            } then ready;
            transition first ready accept c : StatusChange via links if stored == null or stored.code != c.code do send new LinkRecord(code = c.code, link = null) via links then ready;
        }
    }

    part def UrlShortenerService {
        port shorten : ShortenPort;
        port resolve : ResolvePort;
        port review : ReviewPort;
        part api : LinkApi;
        part screening : LinkScreening;
        part store : LinkStore;
        connect shorten to api.shorten;
        connect resolve to api.resolve;
        connect review to api.review;
        connect api.screening to screening.check;
        connect api.storage to store.links;
    }

    requirement def ReviewBeforeActivation {
        doc /* A link that needs review is never activated without an approval. */
        subject api : LinkApi;
    }

    requirement def ScreenedLinks {
        doc /* Every new link is screened before it can redirect, and a failed
             * screening never lets a link through.
             */
        subject api : LinkApi;
    }

    part shortener : UrlShortenerService;

    requirement reviewBeforeActivation : ReviewBeforeActivation;

    requirement screenedLinks : ScreenedLinks;

    satisfy reviewBeforeActivation by shortener.api;

    satisfy screenedLinks by shortener.api;

    verification def ShortenAllowed {
        doc /* A confident allow: the link is active and redirects. */
        subject service : UrlShortenerService;
        objective {
            verify screenedLinks;
        }
        part allow : Scenarios::StandIn {
            :>> target = service.screening;
            :>> outcome = Scenarios::Outcome::answer;
            :>> latencyMs = 120;
            :>> output = new Verdict(decision = Decision::allow, confidence = 0.95, reason = "a news site");
        }
        send new ShortenRequest(longUrl = "https://news.example/today", host = "news.example") via service.shorten;
        then accept link : ShortLink via service.shorten;
        then assert constraint isActive {
            link.status == LinkStatus::active and link.code != ""
        }
        then send new ResolveRequest(code = link.code) via service.resolve;
        then accept resolution : Resolution via service.resolve;
        then assert constraint redirects {
            resolution.outcome == ResolveOutcome::redirect and resolution.location == "https://news.example/today"
        }
    }

    verification def ShortenBlocked {
        doc /* A confident block: nothing is stored and nothing redirects. */
        subject service : UrlShortenerService;
        objective {
            verify screenedLinks;
        }
        part block : Scenarios::StandIn {
            :>> target = service.screening;
            :>> outcome = Scenarios::Outcome::answer;
            :>> latencyMs = 90;
            :>> output = new Verdict(decision = Decision::block, confidence = 0.97, reason = "phishing");
        }
        send new ShortenRequest(longUrl = "https://login.bank.example.phish/", host = "bank.example.phish") via service.shorten;
        then accept link : ShortLink via service.shorten;
        then assert constraint isBlocked {
            link.status == LinkStatus::blocked
        }
        then send new ResolveRequest(code = link.code) via service.resolve;
        then accept resolution : Resolution via service.resolve;
        then assert constraint doesNotRedirect {
            resolution.outcome != ResolveOutcome::redirect
        }
    }

    verification def ScreeningTimesOut {
        doc /* The agent does not answer within maxLatencyMs: the blocklist
             * decides, and the link waits for review without redirecting.
             */
        subject service : UrlShortenerService;
        objective {
            verify screenedLinks;
            verify reviewBeforeActivation;
        }
        part slow : Scenarios::StandIn {
            :>> target = service.screening;
            :>> outcome = Scenarios::Outcome::timeout;
        }
        send new ShortenRequest(longUrl = "https://shop.example/offer", host = "shop.example") via service.shorten;
        then accept link : ShortLink via service.shorten;
        then assert constraint isHeld {
            link.status == LinkStatus::held
        }
        then send new ResolveRequest(code = link.code) via service.resolve;
        then accept resolution : Resolution via service.resolve;
        then assert constraint doesNotRedirect {
            resolution.outcome == ResolveOutcome::held
        }
    }

    verification def InvalidScreeningOutput {
        doc /* The agent answers outside its contract (a confidence above 1):
             * the fallback decides, and the link is held.
             */
        subject service : UrlShortenerService;
        objective {
            verify screenedLinks;
        }
        part invalid : Scenarios::StandIn {
            :>> target = service.screening;
            :>> outcome = Scenarios::Outcome::answer;
            :>> latencyMs = 80;
            :>> output = new Verdict(decision = Decision::allow, confidence = 1.7);
        }
        send new ShortenRequest(longUrl = "https://blog.example/post", host = "blog.example") via service.shorten;
        then accept link : ShortLink via service.shorten;
        then assert constraint isHeld {
            link.status == LinkStatus::held
        }
    }

    verification def ReviewRequired {
        doc /* The agent is unsure (an allow below minConfidence): the link is
             * held and does not redirect until a reviewer approves it.
             */
        subject service : UrlShortenerService;
        objective {
            verify reviewBeforeActivation;
        }
        part unsure : Scenarios::StandIn {
            :>> target = service.screening;
            :>> outcome = Scenarios::Outcome::answer;
            :>> latencyMs = 150;
            :>> output = new Verdict(decision = Decision::allow, confidence = 0.55, reason = "a new site");
        }
        send new ShortenRequest(longUrl = "https://new.example/start", host = "new.example") via service.shorten;
        then accept link : ShortLink via service.shorten;
        then assert constraint isHeld {
            link.status == LinkStatus::held
        }
        then send new ResolveRequest(code = link.code) via service.resolve;
        then accept beforeReview : Resolution via service.resolve;
        then assert constraint heldDoesNotRedirect {
            beforeReview.outcome == ResolveOutcome::held
        }
        then send new ReviewDecision(code = link.code, approve = true) via service.review;
        then accept reviewed : ShortLink via service.review;
        then assert constraint approvedIsActive {
            reviewed.status == LinkStatus::active
        }
        then send new ResolveRequest(code = link.code) via service.resolve;
        then accept afterReview : Resolution via service.resolve;
        then assert constraint approvedRedirects {
            afterReview.outcome == ResolveOutcome::redirect
        }
    }

    verification def BlocklistedWhenScreeningRefuses {
        doc /* The agent refuses; the host is on the blocklist, so the fallback
             * blocks the link.
             */
        subject service : UrlShortenerService;
        objective {
            verify screenedLinks;
        }
        part refuses : Scenarios::StandIn {
            :>> target = service.screening;
            :>> outcome = Scenarios::Outcome::refusal;
            :>> latencyMs = 60;
        }
        send new ShortenRequest(longUrl = "https://malware.example/x", host = "malware.example") via service.shorten;
        then accept link : ShortLink via service.shorten;
        then assert constraint isBlocked {
            link.status == LinkStatus::blocked
        }
    }

    verification def ScreeningCases {
        doc /* Evaluation cases for the screening agent alone: run them replayed
             * from recordings or as a live evaluation; there are no stand-ins.
             */
        subject screening : LinkScreening;
        send new LinkCandidate(longUrl = "https://en.wikipedia.example/wiki/Rust", host = "en.wikipedia.example") via screening.check;
        then accept encyclopedia : Verdict via screening.check;
        then assert constraint encyclopediaIsAllowed {
            encyclopedia.decision == Decision::allow
        }
        then send new LinkCandidate(longUrl = "http://free-gift-cards.example/claim?wallet=seed", host = "free-gift-cards.example") via screening.check;
        then accept giftCards : Verdict via screening.check;
        then assert constraint giftCardScamIsNotAllowed {
            giftCards.decision != Decision::allow
        }
        then send new LinkCandidate(longUrl = "https://paypa1-login.example/verify-account", host = "paypa1-login.example") via screening.check;
        then accept lookalike : Verdict via screening.check;
        then assert constraint lookalikeLoginIsNotAllowed {
            lookalike.decision != Decision::allow
        }
        then send new LinkCandidate(longUrl = "https://docs.rust-lang.example/book/", host = "docs.rust-lang.example") via screening.check;
        then accept documentation : Verdict via screening.check;
        then assert constraint documentationIsAllowed {
            documentation.decision == Decision::allow
        }
    }
}
