# SysML grammar preparation: authority review required

**NOT ADOPTED. Production SysML parsing remains 0/21; lowering and semantics have
not started.** These are concrete proposals for future review, independently of
the pending canonical KerML publication gate.

An offline projection of the pinned final SysML 2.0 grammar recognizes 13 of the
21 unchanged Systems Library documents. Four isolated compatibility hypotheses
recognize 21/21 with the existing Python grammar-context recognizer, without
recovery. This establishes an implementation inventory, not runtime parsing,
operator precedence, canonical lowering, or semantic acceptance.

The differences below were checked visually in rendered pages of the exact
pinned `SysML.pdf`, independently of text extraction. Page numbers are one-based
PDF pages, followed by printed pages in parentheses.

| Pending review | Printed production and anchor | Actual original corpus witness |
| --- | --- | --- |
| Allocation dispatch | `DefinitionElement`, §8.2.2.5.2, p. 200 (168), omits `AllocationDefinition`, which is defined in §8.2.2.15 | `Allocations.sysml`: `allocation def Allocation :> BinaryConnection` |
| Case returns | `CaseBodyItem`, §8.2.2.22, p. 221 (189), has no return-member alternative | `Cases.sysml`: `return ref result[0..*]`; `VerificationCases.sysml`: `return verdict : VerdictKind :>> result` |
| End usage prefixes | `DefaultReferenceUsage`, §8.2.2.6.3, p. 202 (170), starts with `RefPrefix`; `OccurrenceUsagePrefix`, §8.2.2.9.2, p. 206 (174), starts with `BasicUsagePrefix` | Bare `end` in Connections; `end occurrence` in Flows; `end port` in Interfaces; `end touchesToo [0..*] item touchedItemToo` in Items |
| Satisfy prefix | `SatisfyRequirementUsage`, §8.2.2.21.2, p. 221 (189), requires both `assert` and `not` | `Views.sysml`: `satisfy requirement viewpointConformance by that` |

The exact proposed recognition edits are:

```ebnf
# Append one alternative to each existing rule:
DefinitionElement = ... | AllocationDefinition;
CaseBodyItem = ... | ReturnParameterMember;

# Replace DefaultReferenceUsage's RHS:
DefaultReferenceUsage = (EndUsagePrefix | RefPrefix) Usage;

# Replace only OccurrenceUsagePrefix's first nonterminal:
OccurrenceUsagePrefix = UnextendedUsagePrefix ...;

# Replace only SatisfyRequirementUsage's prefix:
SatisfyRequirementUsage =
    OccurrenceUsagePrefix 'assert'? (isNegated ?= 'not')? 'satisfy' ...;
```

All omitted tails remain exactly as printed. These five edits are grouped into
four authority decisions. They neither revise the original sources nor adopt a
new operational profile. The earlier single-gap finding in
[authority-decision.json](authority-decision.json) remains unchanged; this review
supplements it.

The draft contains 349 printed SysML rules, including six special lexical rules:
268 new names and 81 names also present in KerML. Adding 186 inherited KerML rules
gives 535 combined rules. The isolated corpus derivations use 291 named
productions: 169 new SysML, 62 same-named SysML, and 60 unchanged KerML. Identical
recognition syntax does not imply identical canonical lowering; SysML's declared
metaclass and membership synthesis remain authoritative.

[The review record](grammar-preparation-review.json) retains the pinned PDF and
KPAR hashes, full affected printed rules, exact UTF-8 source ranges, all 21
outcomes, and raw-evidence hashes. [The command record](grammar-preparation-commands.json)
records three actual commands, outputs and exit code 0. An analysis command
returning 0 means its inventory completed, not that its input grammar accepted
all documents. Raw extraction, EBNF, probes and page images remain ignored under
`verification/generated/sysml-grammar-preparation/`.
