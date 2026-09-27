package UrlShortener {
    doc /* A small web service that turns long URLs into short codes,
         * redirects visitors to the long URL and counts clicks per code.
         * REALIGNMENT Scenario A.
         * Test fixture: a copy with one deliberate error. */

    private import ScalarValues::*;

    // Items exchanged across the interfaces.
    item def ShortenRequest {
        attribute longUrl : String;
    }
    item def ShortLink {
        attribute code : String;
        attribute longUrl : String;
    }
    item def ResolveRequest {
        attribute code : String;
    }
    item def Redirect {
        attribute location : String;
    }
    item def ClickEvent {
        attribute code : String;
    }
    item def StatsRequest {
        attribute code : String;
    }
    item def LinkStats {
        attribute code : String;
        attribute clicks : Natural;
    }

    // Ports.
    port def ShortenPort {
        in item request : ShortenRequest;
        out item link : ShortLink;
    }
    port def ResolvePort {
        in item request : ResolveRequest;
        out item redirect : Redirect;
    }
    port def StatsPort {
        in item request : StatsRequest;
        out item stats : LinkStats;
    }
    port def LinkStorePort {
        in item save : ShortLink;
        in item lookup : ResolveRequest;
        out item found : ShortLink;
    }
    port def ClickPort {
        out item click : ClickEvent;
    }

    // Interfaces between the parts.
    interface def LinkStorage {
        end port client : ~LinkStorePort;
        end port store : LinkStorePort;
    }
    interface def ClickReporting {
        end port source : ClickPort;
        end port sink : ~ClickPort;
    }

    // Parts.
    part def HttpApi {
        doc /* Accepts shorten, resolve and stats requests over HTTP. */
        port shorten : ShortenPort;
        port resolve : ResolvePort;
        port stats : ~StatsPort;
        port storage : ~LinkStorePort;
        port clicks : ClickPort;
    }
    part def LinkStore {
        doc /* Persists short links and looks them up by code. */
        port links : LinkStorePort;
        attribute capacity : Natural;
    }
    part def SqlLinkStore :> LinkStore {
        doc /* A link store backed by an SQL database. */
        attribute :>> capacity = 1000000;
    }
    part def ClickStats {
        doc /* Counts clicks per short code and reports them. */
        // Deliberate error: the conjugation `~` is missing, so both ends of
        // clickReporting send clicks and neither receives them.
        port clicks : ClickPort;
        port stats : StatsPort;
    }
    part def UrlShortenerService {
        part api : HttpApi;
        part store : SqlLinkStore;
        part clickStats : ClickStats[1];
        interface storage : LinkStorage connect api.storage to store.links;
        interface clickReporting : ClickReporting connect api.clicks to clickStats.clicks;
        connection statsQuery connect api.stats to clickStats.stats;
    }

    // Requirements.
    requirement def UniqueCodes {
        doc /* Every short code maps to exactly one long URL. */
        subject store : LinkStore;
    }
    requirement def FastRedirect {
        doc /* A redirect is answered within the latency budget. */
        subject api : HttpApi;
        attribute maxLatencyMs : Natural;
    }

    part shortener : UrlShortenerService;
    requirement uniqueCodes : UniqueCodes;
    requirement fastRedirect : FastRedirect {
        attribute :>> maxLatencyMs = 50;
    }
    satisfy uniqueCodes by shortener.store;
    satisfy fastRedirect by shortener.api;
}
