# Real immutable revision read profile, before closure/audit optimization

Passed with the installed authenticated runtime and a SQLite backup of the real
Agentique repository (unchanged revision/branch rows). The exact revision is
9539bfed-f725-43d9-b5f8-8254040e7d42: 76,153 canonical elements, 791 local elements.
The executable contains retained immutable query contexts and the existing bounded
projection/Inspector caches. It predates the new audit/producer/cache-rebind drafts.
Build provenance: acceptance-profile-build-02, starting source bd72aee4; exact
executable SHA-256 and launch source are recorded in measurement.json.

| Measured boundary | Wall time |
| --- | ---: |
| Runtime location/transport setup | 881 ms |
| KerML authentication and load | 37.242 s |
| Systems authentication and load | 38.716 s |
| Repository open | 103 ms |
| Source/identity loading | 4.7 ms |
| Cache blob load/authentication | 38.6 ms |
| Cache decode and frontier digest | 248 ms |
| Revision restoration, including receipt checks | 116.734 s |
| Warm project open, inclusive total | 193.691 s |
| First System projection after restoration | 64.1 ms |
| Graph overview | 63.2 ms |
| Focused System projection | 323.3 ms |
| Focused Graph projection | 56.0 ms |
| Requirements projection (4 nodes/3 edges) | 56.6 ms |
| Inspector first read (ModelRepository) | 252.0 ms |
| Inspector repeat | 0.22 ms |
| Repeated projections | 0.93–1.00 ms |

Compilation inside cache restoration took 110.341 s: source preparation 15.262 s,
strict kernel validation 0.586 s, final closure 38.607 s, final references 7.857 s,
effective audit 47.964 s. Nested preparation phases must not be added twice.
Cache use is authenticated and reported true, yet 332 producer subjects and all
791 local audit subjects were evaluated. This establishes that decode is not the
restoration bottleneck. Ordinary loaded reads do not repeat this full audit.

The first Inspector's dominating measured cost was type/effective-feature queries
(195 ms for 91 effective features), followed by relationship queries (36 ms).
It misses the aspirational 100 ms first-read target; its exact DTO repeat is cached.

This is one Windows sample under substantial system memory pressure, not an SLA or
distribution. Peak process working set was 5,408,374,784 bytes. No competing semantic
process or compiler ran during this profile. UI queue, layout, scene construction,
GPU upload and actual displayed-frame time are excluded and remain native-journey
measurements. Warm open has not improved; loaded focus has improved from the
previous mission's roughly 3.55 s to this 323 ms observation.
