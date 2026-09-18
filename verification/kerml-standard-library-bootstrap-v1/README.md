# KerML standard-library bootstrap v1

Base: `1eb1424` (fetched `origin/main`, 2026-09-18). Initial working tree clean.
Branch: `semantics/kerml-standard-library-bootstrap-v1`.

Evidence directories are append-only. `run.mjs` records commands, complete outputs,
actual exit codes and elapsed time. Earlier milestone evidence and the exact
pinned source/artifact files are retained. No Systems Library semantic work or
platform work is part of this milestone.

## Gate 0

The grammar-context inventory recognizes all 36 KerML source documents and
records 199 production kinds with source ranges and examples. It is generated
from exact verified source/lexer output using the maintained KerML 1.0 grammar
projection. The first-derivation reference tree does not claim precedence AST,
canonical lowering, resolution or semantic validation. ADR 0013 records the
parser architecture comparison and each grammar transcription disposition.

`gate-0/results.json` records all six commands at exit zero: inventory currentness,
recognizer tests, existing syntax/source tests, formatting and both complete
structural runtime gates. This establishes only Gate 0.

The supplied PDF extraction is retained as reading evidence. The grammar input
file contains the verified exact sources and token ranges used during initial
analysis; reproduction normally obtains fresh inputs directly from the verified
offline loader. No acquisition occurs in either path.

Remaining gates: richer production CST, precedence, recovery, corpus frontend,
complete canonical lowering, import/alias/visibility resolution, atomic library
publication, validated bindings, loaded context identity, semantic validation,
quality gate, authored integration, dependency audit and stress/review acceptance.

## Gate 1

`agq-kerml-syntax::production` now supplies the grammar frontend, a lossless token
partition and typed production arena, explicit recovery and published grammar
discrepancies. Runtime tables are generated and checked in; Table 6 precedence and
associativity have focused structural tests. All source traversal and tree
materialization are iterative and chart work is bounded. Syntax IDs are deterministic
within an exact source revision; edits conservatively retain unaffected ranges.

The existing bounded authored adapter remains available during lowering migration.
It does not gain canonical support merely because a production tree can recognize
additional syntax. `gate-1/results.json` records seven successful commands,
including focused production tests, authored regressions, Clippy, strict Rustdoc
and both runtime checks. The whole corpus frontend report is the next gate.
