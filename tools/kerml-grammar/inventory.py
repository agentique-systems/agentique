"""Offline grammar-context inventory, using verified source/lexer output.

Earley recognition intentionally permits the indirect left recursion in 8.2.5.8.
This is an analysis tool, not semantic loading or the recoverable Rust frontend.
Regex is used only to read the EBNF notation, never to classify library constructs.
"""
from __future__ import annotations

import argparse
from collections import defaultdict
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
GRAMMAR = ROOT / "standards/grammar/kerml-1.0.ebnf"


class Grammar:
    def __init__(self, text, *, unresolved=()):
        self.rules = []
        self.by_name = defaultdict(list)
        self.names = []
        self.synthetic = 0
        self.tokens = re.findall(r"'[^']*'|[A-Za-z_][A-Za-z_0-9]*|[=;()|?*+]", re.sub(r"(?m)^#.*$", "", text))
        self.pos = 0
        while self.pos < len(self.tokens):
            name = self.take()
            assert name not in self.names, name
            self.names.append(name)
            assert self.take() == "="
            for rhs in self.choice():
                self.add(name, rhs)
            assert self.take() == ";", name
        missing = {s for _, rhs in self.rules for s in rhs if not s.startswith("'") and s not in self.by_name}
        assert missing == {"NAME", "STRING_VALUE", "DECIMAL_VALUE", "EXPONENTIAL_VALUE", "REGULAR_COMMENT"} | set(unresolved), missing
        # An undefined published nonterminal has no alternatives. It cannot
        # consume a token or match epsilon; callers must name every such gap.
        for name in unresolved:
            self.by_name[name] = []
        self.keywords = {s[1:-1] for _, rhs in self.rules for s in rhs if s.startswith("'") and s[1:-1].isalpha()}
        # KerML 1.0 8.2.2.6: all reserved names, including those not used by a production.
        lexer = (ROOT / "crates/kerml-syntax/src/lexer.rs").read_text()
        self.keywords.update(lexer.split('const KEYWORDS: &str = "')[1].split('";')[0].split())

    def take(self):
        value = self.tokens[self.pos]
        self.pos += 1
        return value

    def add(self, name, rhs):
        self.by_name[name].append(len(self.rules))
        self.rules.append((name, tuple(rhs)))

    def group(self, alternatives):
        name = f"_group_{self.synthetic}"
        self.synthetic += 1
        for rhs in alternatives:
            self.add(name, rhs)
        return name

    def choice(self):
        alternatives = [[]]
        while self.tokens[self.pos] not in (")", ";"):
            token = self.take()
            if token == "|":
                alternatives.append([])
                continue
            if token == "(":
                token = self.group(self.choice())
                assert self.take() == ")"
            if self.tokens[self.pos] in ("?", "*", "+"):
                repeat = self.take()
                name = self.group([[]] if repeat != "+" else [[token]])
                self.add(name, [token] if repeat == "?" else [name, token])
                token = name
            alternatives[-1].append(token)
        return alternatives


def recognize(grammar, tokens):
    """Finite chart; completed production nodes retain parent/child grammar context.

    First derivation wins in grammar order. This recognition inventory is NOT an
    operator-precedence AST; runtime expression construction must apply Table 6.
    """
    charts = [{} for _ in range(len(tokens) + 1)]
    queues = [[] for _ in charts]
    waiting = [defaultdict(list) for _ in charts]
    completed = {}
    nodes = []

    def insert(end, key, children):
        if key not in charts[end]:
            charts[end][key] = children
            queues[end].append(key)

    for rule in grammar.by_name["RootNamespace"]:
        insert(0, (rule, 0, 0), ())
    farthest = 0
    for end, queue in enumerate(queues):
        if queue:
            farthest = end
        index = 0
        while index < len(queue):
            rule, dot, start = key = queue[index]
            index += 1
            name, rhs = grammar.rules[rule]
            children = charts[end][key]
            if dot == len(rhs):
                complete_key = (name, start, end)
                if complete_key in completed:
                    continue
                node = len(nodes)
                nodes.append((name, start, end, children))
                completed[complete_key] = node
                for parent in waiting[start][name]:
                    insert(end, (parent[0], parent[1] + 1, parent[2]), charts[start][parent] + (node,))
            else:
                symbol = rhs[dot]
                if symbol in grammar.by_name:
                    waiting[end][symbol].append(key)
                    for child in grammar.by_name[symbol]:
                        insert(end, (child, 0, end), ())
                    empty = completed.get((symbol, end, end))
                    if empty is not None:
                        insert(end, (rule, dot + 1, start), children + (empty,))
                elif end < len(tokens):
                    kind, text, *_ = tokens[end]
                    matched = text == symbol[1:-1] if symbol.startswith("'") else (
                        kind == symbol or (symbol == "NAME" and kind == "word" and text not in grammar.keywords))
                    if matched:
                        insert(end + 1, (rule, dot + 1, start), children)
    root = completed.get(("RootNamespace", 0, len(tokens)))
    expected = sorted({grammar.rules[r][1][d] for r, d, _ in charts[farthest] if d < len(grammar.rules[r][1]) and grammar.rules[r][1][d] not in grammar.by_name})
    return root, nodes, farthest, expected


def main():
    args = argparse.ArgumentParser()
    args.add_argument("--write", action="store_true")
    args.add_argument("--inputs", type=Path)
    args.add_argument("--only")
    options = args.parse_args()
    if options.inputs:
        inputs = json.loads(options.inputs.read_bytes())
    else:
        process = subprocess.run(["cargo", "run", "--locked", "--offline", "-p", "agq-standard-libraries", "--example", "grammar_inputs"], cwd=ROOT, stdout=subprocess.PIPE, check=True)
        inputs = json.loads(process.stdout)
    grammar = Grammar(GRAMMAR.read_text())
    occurrences = defaultdict(list)
    reports = []
    sources = {d["file"]: d["source"].encode() for d in inputs["documents"]}
    for doc in inputs["documents"]:
        if options.only and options.only not in doc["file"]:
            continue
        tokens = doc["tokens"]
        root, nodes, farthest, expected = recognize(grammar, tokens)
        print(doc["file"], "recognized" if root is not None else f"FAILED at {tokens[farthest:farthest+4]} expected {expected}", flush=True)
        found = set()
        if root is not None:
            pending = [root]
            while pending:
                n = pending.pop()
                name, start, end, children = nodes[n]
                pending.extend(reversed(children))
                if name.startswith("_"):
                    continue
                found.add(name)
                lo = tokens[start][2] if start < len(tokens) else len(doc["source"].encode())
                hi = tokens[end - 1][3] if end > start else lo
                occurrences[name].append({"document": doc["file"], "byte_range": [lo, hi]})
        reports.append({"file": doc["file"], "sha256": doc["sha256"], "grammar_status": "recognized" if root is not None else "unrecognized", "production_count": len(found), "productions": sorted(found), "baseline_blocking_frontend_productions": sorted(found - EXISTING), "current_blocking_frontend_productions": [], "failure": None if root is not None else {"token": farthest, "expected": expected}})
    inventory = {
        "format": "agentique-library-syntax-coverage/2", "library_set": inputs["library_set"],
        "scope": "36 exact pinned KerML documents; Systems Library remains outside this milestone",
        "authority": {"specification": "KerML 1.0", "sections": ["8.2.3", "8.2.4", "8.2.5"], "grammar": str(GRAMMAR.relative_to(ROOT)).replace('\\', '/'), "grammar_sha256": hashlib.sha256(GRAMMAR.read_bytes()).hexdigest()},
        "method": "Earley grammar-context recognition over verified lossless lexer tokens; no name resolution or semantic loading. First derivation in grammar order, not a precedence AST.",
        "status": "production-inventory-complete" if all(d["failure"] is None for d in reports) and len(reports) == 36 else "production-inventory-incomplete",
        "documents": reports,
        "productions": [{"production": name, "count": len(items), "documents": sorted({i["document"] for i in items}),
                         "source_ranges": {file: [str(i['byte_range'][0]) + '..' + str(i['byte_range'][1]) for i in items if i['document'] == file] for file in sorted({i['document'] for i in items})},
                         "examples": [{**i, "text": sources[i['document']][i['byte_range'][0]:i['byte_range'][1]].decode()[:240]} for i in items[:3]],
                         "parser_support": "agq-kerml-syntax::production; complete pinned corpus verified",
                         "ast_cst_support": "typed lossless production tree; Table 6 expression precedence",
                         "lowering_support": "traversed by canonical kernel construction; strict library publication incomplete",
                         "resolution_support": "semantic namespace/import/alias/visibility/inheritance queries implemented; corpus reference completeness not accepted",
                         "semantic_interpretation_support": "candidate structural interpretation; full library validation incomplete; execution unsupported"} for name, items in sorted(occurrences.items())],
        "unused_grammar_productions": sorted(set(grammar.names) - occurrences.keys()),
        "range_encoding": "UTF-8 half-open start..end; empty productions have zero-width ranges; examples truncate after 240 characters, ranges do not",
    }
    path = ROOT / "standards/library-syntax-coverage.json"
    data = (json.dumps(inventory, indent=2, ensure_ascii=False) + "\n").encode()
    if options.write:
        path.write_bytes(data)
    elif not options.only and path.read_bytes() != data:
        raise SystemExit("Stale production inventory")
    if inventory["status"] != "production-inventory-complete" and not options.only:
        raise SystemExit(1)


EXISTING = set("RootNamespace Namespace NamespaceDeclaration NamespaceBody NamespaceBodyElement NamespaceMember NonFeatureMember NamespaceFeatureMember MemberElement NonFeatureElement FeatureElement Type TypePrefix TypeDeclaration TypeBody TypeBodyElement FeatureMember OwnedFeatureMember Feature FeatureDeclaration FeatureIdentification SpecializationPart OwnedSpecialization GeneralType QualifiedName Typings TypedBy OwnedFeatureTyping Subsettings Subsets OwnedSubsetting Redefinitions Redefines OwnedRedefinition TYPED_BY SPECIALIZES SUBSETS REDEFINES".split())

if __name__ == "__main__":
    main()
