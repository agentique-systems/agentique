"""Compile maintained grammar data into offline Rust recognition tables.

The expression transformation is the precedence/associativity contract in
KerML 1.0 Table 6. Public node kinds retain the normative production names.
"""
from pathlib import Path
import json
import subprocess
import sys
from inventory import Grammar, GRAMMAR, ROOT


def runtime_grammar(grammar=None):
    if grammar is None:
        grammar = Grammar(GRAMMAR.read_text())
    removed = {"OwnedExpression", "ConditionalExpression", "ConditionalBinaryOperatorExpression", "BinaryOperatorExpression", "UnaryOperatorExpression", "ClassificationExpression", "MetaclassificationExpression", "ExtentExpression"}
    old = grammar.rules
    grammar.rules = []
    grammar.by_name.clear()
    for name, rhs in old:
        if name not in removed:
            grammar.add(name, rhs)
    kinds = {name: name for name in grammar.names}
    serial = 0

    def node(kind, rhs):
        nonlocal serial
        name = f"_precedence_node_{serial}"
        serial += 1
        kinds[name] = kind
        grammar.add(name, rhs)
        return name

    def argument(target, conditional=False):
        target = node("OwnedExpression", [target])
        wrappers = ["ArgumentValue", "Argument", "ArgumentMember"]
        if conditional:
            wrappers = ["OwnedExpressionMember", "OwnedExpressionReference", "ArgumentExpressionValue", "ArgumentExpression", "ArgumentExpressionMember"]
        for kind in wrappers:
            target = node(kind, [target])
        return target

    levels = [
        ["??"], ["implies"], ["|", "or"], ["xor"], ["&", "and"],
        ["==", "!=", "===", "!=="], None, ["<", ">", "<=", ">="],
        [".."], ["+", "-"], ["*", "/", "%"], ["^", "**"],
    ]
    grammar.add("OwnedExpression", ["ConditionalExpression"])
    grammar.add("OwnedExpression", ["_precedence_0"])
    grammar.add("ConditionalExpression", ["'if'", "ArgumentMember", "'?'", "ArgumentExpressionMember", "'else'", "ArgumentExpressionMember", "EmptyResultMember"])
    for level, operators in enumerate(levels):
        current = f"_precedence_{level}"
        higher = f"_precedence_{level + 1}"
        grammar.add(current, [higher])
        if operators is None:
            for operator in ["istype", "hastype", "@", "as"]:
                op = node("CastOperator" if operator == "as" else "ClassificationTestOperator", [repr(operator)])
                target = "TypeResultMember" if operator == "as" else "TypeReferenceMember"
                grammar.add(current, [node("ClassificationExpression", [argument(current), op, target, "EmptyResultMember"])])
                grammar.add(current, [node("ClassificationExpression", [op, target, "EmptyResultMember"])])
            for op, target in [("MetaclassificationTestOperator", "TypeReferenceMember"), ("MetaCastOperator", "TypeResultMember")]:
                grammar.add(current, [node("MetaclassificationExpression", ["MetadataArgumentMember", op, target, "EmptyResultMember"])])
            continue
        for operator in operators:
            conditional = operator in {"??", "implies", "or", "and"}
            kind = "ConditionalBinaryOperatorExpression" if conditional else "BinaryOperatorExpression"
            op_kind = "ConditionalBinaryOperator" if conditional else "BinaryOperator"
            op = node(op_kind, [repr(operator)])
            left, right = (higher, current) if operator in {"^", "**"} else (current, higher)
            grammar.add(current, [node(kind, [argument(left), op, argument(right, conditional), "EmptyResultMember"])])
    grammar.add("_precedence_12", ["UnaryOperatorExpression"])
    grammar.add("_precedence_12", ["_extent"])
    grammar.add("UnaryOperatorExpression", ["UnaryOperator", argument("_precedence_12"), "EmptyResultMember"])
    grammar.add("_extent", ["ExtentExpression"])
    grammar.add("_extent", ["PrimaryExpression"])
    grammar.add("ExtentExpression", ["'all'", "TypeReferenceMember"])
    return grammar, kinds


def variant(name):
    return "".join(p.title() for p in name.split("_")) + "Keyword" if name.isupper() else name


def tables(grammar, kinds):
    """Encode either dialect using the same production/terminal contracts."""
    symbols = sorted(grammar.by_name)
    ids = {name: i for i, name in enumerate(symbols)}
    lexicals = {"NAME": "Name", "STRING_VALUE": "String", "REGULAR_COMMENT": "Comment", "DECIMAL_VALUE": "Decimal", "EXPONENTIAL_VALUE": "Exponential"}
    output = [f'pub(super) const ROOT: u16 = {ids["RootNamespace"]};', f'pub(super) const SYMBOL_COUNT: usize = {len(ids)};', 'pub(super) const RULES: &[Rule] = &[']
    for name, rhs in grammar.rules:
        encoded = []
        for s in rhs:
            if s.startswith("'"):
                encoded.append(f'Symbol::Text("{s[1:-1]}")')
            elif s in ids:
                encoded.append(f'Symbol::Nonterminal({ids[s]})')
            else:
                encoded.append(f'Symbol::{lexicals[s]}')
        kind = f'Some(Production::{variant(kinds[name])})' if name in kinds else 'None'
        output.append(f'Rule {{ lhs: {ids[name]}, kind: {kind}, rhs: &[{", ".join(encoded)}] }},')
    output += ['];', 'pub(super) const KEYWORDS: &[&str] = &[']
    output += [f'"{word}",' for word in sorted(grammar.keywords)]
    output += ['];']
    return output


def format_rust(output):
    return subprocess.run(['rustfmt', '--edition', '2024', '--emit', 'stdout'], input='\n'.join(output).encode(), stdout=subprocess.PIPE, check=True).stdout.replace(b'\r\n', b'\n')


def main():
    grammar, kinds = runtime_grammar()
    # One production vocabulary is shared by both dialects. KerML rules and
    # precedence remain unchanged; SysML overrides are separate runtime tables.
    sysml = json.loads((ROOT / 'standards/grammar/sysml-2.0-source.json').read_text(encoding='utf-8'))
    public = sorted(set(kinds.values()) | {rule['name'] for rule in sysml['rules']})
    output = ['// @generated by tools/kerml-grammar/generate.py; do not edit.', 'use super::{Rule, Symbol};',
              '/// Shared KerML 1.0 / SysML 2.0 textual production, independent of metaclasses.',
              '#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]', 'pub enum Production {']
    output += [f'    {variant(name)},' for name in public]
    output += ['}', 'impl Production {', '/// Normative grammar production name.', "pub fn name(self) -> &'static str { match self {"]
    output += [f'Self::{variant(name)} => "{name}",' for name in public]
    output += ['}}}']
    data = format_rust(output + tables(grammar, kinds))
    path = ROOT / 'crates/kerml-syntax/src/production/generated.rs'
    if '--write' in sys.argv:
        path.parent.mkdir(exist_ok=True)
        path.write_bytes(data)
    else:
        assert path.read_bytes().replace(b'\r\n', b'\n') == data, 'stale grammar tables'
    print(f'{len(public)} production kinds; {len(grammar.rules)} grammar alternatives; tables current')


if __name__ == '__main__':
    main()
