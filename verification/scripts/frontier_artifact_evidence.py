"""Authenticate final unaccepted checkpoints and compare selected graph evidence.

The historical graph digest covers aggregate proofs/searches, but not selected
ordered-reference contributions. Resolve their archive intern pools by content.
Only small per-proof hashes are retained; scheduler transport rows are streamed.
"""
import hashlib
import json
from pathlib import Path
import re
import zipfile

SPECIAL = re.compile(rb'["\\{}\[\]]')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def pin(value):
    require(isinstance(value, list) and len(value) == 32
            and all(type(x) is int and 0 <= x <= 255 for x in value), "invalid digest")
    return bytes(value).hex()


def content_hash(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":"),
                                     ensure_ascii=True).encode()).hexdigest()


class StateReader:
    """Bounded JSON field extraction; skipped payloads still enter the byte pin.

    This is not an authority decoder. The authenticated scheduler state supplies
    only report bindings. The Rust resume path remains its semantic validator.
    """
    def __init__(self, source):
        self.source, self.buffer, self.offset = source, b"", 0
        self.digest = hashlib.sha256()

    def peek(self):
        if self.offset == len(self.buffer):
            self.buffer = self.source.read(64 * 1024)
            self.digest.update(self.buffer)
            self.offset = 0
        return self.buffer[self.offset:self.offset + 1]

    def take(self):
        result = self.peek()
        require(result, "truncated checkpoint state")
        self.offset += 1
        return result

    def whitespace(self):
        while self.peek() and self.peek() in b" \t\r\n":
            self.offset += 1

    def value(self, capture=False):
        self.whitespace()
        first = self.peek()
        require(first, "missing state value")
        if not capture:
            if first not in (b"{", b"["):
                self.value(True)
                return None
            self.take()
            stack = [b"}" if first == b"{" else b"]"]
            quoted = escaped = False
            while stack:
                require(self.peek(), "truncated skipped state value")
                match = SPECIAL.search(self.buffer, self.offset)
                if match is None:
                    self.offset = len(self.buffer)
                    escaped = False
                    continue
                if match.start() > self.offset:
                    escaped = False
                byte = match.group()
                self.offset = match.end()
                if quoted:
                    if escaped:
                        escaped = False
                    elif byte == b"\\":
                        escaped = True
                    elif byte == b'"':
                        quoted = False
                elif byte == b'"':
                    quoted = True
                elif byte in (b"{", b"["):
                    stack.append(b"}" if byte == b"{" else b"]")
                elif byte in (b"}", b"]"):
                    require(stack.pop() == byte, "unbalanced skipped state value")
            return None
        output = bytearray() if capture else None
        stack, quoted, escaped = [], False, False
        while self.peek():
            byte = self.peek()
            if not quoted and not stack and byte in b",}:] \t\r\n":
                break
            byte = self.take()
            if capture:
                output.extend(byte)
                require(len(output) <= 64 * 1024 * 1024, "state field exceeds limit")
            if quoted:
                if escaped:
                    escaped = False
                elif byte == b"\\":
                    escaped = True
                elif byte == b'"':
                    quoted = False
            elif byte == b'"':
                quoted = True
            elif byte in (b"{", b"["):
                stack.append(b"}" if byte == b"{" else b"]")
            elif byte in (b"}", b"]"):
                require(stack and stack.pop() == byte, "unbalanced state value")
            if not quoted and not stack and first in (b'"', b"{", b"["):
                break
        require(not quoted and not stack, "truncated state value")
        return json.loads(output) if capture else None

    def fields(self, wanted, objects=None):
        objects = objects or {}
        self.whitespace()
        require(self.take() == b"{", "state object expected")
        found = {}
        seen = set()
        self.whitespace()
        if self.peek() == b"}":
            self.take()
            return found
        while True:
            key = self.value(True)
            require(isinstance(key, str) and key not in seen, "invalid/duplicate state key")
            seen.add(key)
            self.whitespace()
            require(self.take() == b":", "state field separator")
            if key in objects:
                found[key] = self.fields(objects[key])
            elif key in wanted:
                found[key] = self.value(True)
            else:
                self.value()
            self.whitespace()
            delimiter = self.take()
            if delimiter == b"}":
                return found
            require(delimiter == b",", "state object delimiter")



def graph_evidence(source):
    """Hash decoded graph bytes and selected support without retaining the graph."""
    proofs, searches, previous = [], [], None
    rows = hashlib.sha256(b"agq-selected-reference-evidence/1\0")
    graph_hash, payload_hash, count, ended = hashlib.sha256(), hashlib.sha256(), 0, False
    header = None
    while line := source.readline(64 * 1024 * 1024 + 1):
        require(len(line) <= 64 * 1024 * 1024, "graph row exceeds limit")
        require(not ended, "graph data after End")
        graph_hash.update(line)
        value = json.loads(line)
        if header is None:
            header = value
        else:
            payload_hash.update(line)
        if value == "End":
            ended = True
            continue
        if value == "Overlay":
            continue
        require(isinstance(value, dict) and len(value) == 1, "invalid graph row")
        if "Proof" in value:
            proofs.append(content_hash(value["Proof"]))
        elif "Search" in value:
            searches.append(content_hash(value["Search"]))
        elif "ReferenceContribution" in value:
            row = value["ReferenceContribution"]
            key = (row["element"], row["property"], row["target"])
            require(previous is None or previous < key,
                    "unordered/duplicate reference contribution")
            previous = key
            for field, table in (("proof", proofs), ("searches", searches)):
                index = row[field]
                require(type(index) is int and 0 <= index < len(table),
                        "invalid selected evidence pool index")
                row[field] = table[index]
            require(type(row["position"]) is int and row["position"] >= 0,
                    "invalid selected position")
            rows.update(bytes.fromhex(content_hash(row)))
            count += 1
    require(ended, "missing graph End")
    return {"graph_sha256": graph_hash.hexdigest(), "graph_payload_sha256": payload_hash.hexdigest(),
            "header": header, "contributions": count, "selected_reference_digest": rows.hexdigest()}

def checkpoint_evidence(report):
    latest = report["checkpoint_session"]["latest"]
    path = Path(latest["journal"])
    expected = latest["sha256"]
    require(isinstance(expected, str) and re.fullmatch("[0-9a-f]{64}", expected),
            "invalid independent journal pin")
    raw = path.read_bytes()
    require(hashlib.sha256(raw).hexdigest() == expected, "checkpoint journal hash")
    journal = json.loads(raw)
    require(journal["format"] == "agq-unaccepted-publication-frontier/1", "journal format")
    source_identity = pin(journal["source_identity"])
    require(journal["entries"], "empty checkpoint journal")
    entry = journal["entries"][-1]
    archive_pin = pin(entry["archive_sha256"])
    archive_path = path.parent / f"{archive_pin}.zip"
    with archive_path.open("rb") as handle:
        require(hashlib.file_digest(handle, "sha256").hexdigest() == archive_pin,
                "checkpoint archive hash")
        handle.seek(0)
        with zipfile.ZipFile(handle) as archive:
            require(sorted(archive.namelist()) == ["graph.jsonl", "state.json"],
                    "unexpected/duplicate checkpoint archive entries")
            with archive.open("state.json") as source:
                reader = StateReader(source)
                state = reader.fields({"converged", "model_digest", "context_contract"},
                                      {"certificate": {"receipt"}})
                reader.whitespace()
                require(not reader.peek(), "trailing state bytes")
                require(reader.digest.hexdigest() == pin(entry["state_sha256"]),
                        "decoded checkpoint state hash")
            require(state["converged"] is True, "final checkpoint is not converged")
            certificate = report["producer_closure"]
            require(state["model_digest"] == certificate["model_digest"], "state graph binding")
            require(state["context_contract"] == certificate["context_contract_digest"],
                    "state context binding")
            receipt = state["certificate"]["receipt"]
            for source_key, report_key in (("model_digest", "model_digest"),
                    ("registry_digest", "producer_registry_digest"),
                    ("context_contract_digest", "context_contract_digest"), ("digest", "digest")):
                require(receipt[source_key] == certificate[report_key],
                        f"checkpoint receipt binding: {source_key}")
            with archive.open("graph.jsonl") as source:
                graph = graph_evidence(source)
            require(graph["graph_sha256"] == pin(entry["graph_sha256"]),
                    "decoded checkpoint graph hash")
    return {"journal_sha256": expected, "archive_sha256": archive_pin,
            "source_identity": source_identity, "certificate_receipt_digest": content_hash(receipt), **graph}
