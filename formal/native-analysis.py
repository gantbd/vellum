"""Serialized live native verification host; pipe protocol is not a product ABI."""
import importlib.util
import json
import pathlib
import sys

sys.dont_write_bytecode = True
directory = pathlib.Path(sys.argv[1])
spec = importlib.util.spec_from_file_location("reference", directory / "tree-sitter-reference.py")
reference = importlib.util.module_from_spec(spec)
spec.loader.exec_module(reference)
catalog = json.loads((directory / "native-analysis.json").read_text())
expected = json.loads((directory / "tree-sitter-reference.json").read_text())["environment"]
records = {}
issued = 0
messages = 0
last_serial = 0


def facts(forest):
    return {key: (reference.signature(tree), [n.has_changes for n in reference.walk(tree.root_node)]) for key, tree in forest.items()}


def failure(reason="unsupported"):
    return {"kind": "failure", "reason": reason}


def execute(request):
    global issued
    capture, handle = request["capture"], request["base"]
    context, source_plan = capture["context"], capture["input"]
    source = reference.source_bytes(source_plan["source"])
    if len(records) >= catalog["maxRecords"] or issued >= catalog["maxIssued"]:
        return failure("exhausted")
    if context["languageProfile"] != "pinned-markdown-javascript-reference/1" or any(context[key] != 1 for key in ("grammarGeneration", "discoveryQueryGeneration", "registryGeneration", "configurationGeneration")):
        return failure()
    if str(context["highlightQueryGeneration"]) not in catalog["queryGenerations"]:
        return failure("invalid-query")
    mode = source_plan["mode"]
    if source_plan["profile"] != "tree-sitter-utf8-source/1" or mode not in ("fresh", "incremental", "unchanged", "reuse-parse") or source_plan["toRevision"] != context["sourceRevision"]:
        return failure()
    base = records.get(handle) if type(handle) is int else None
    if mode == "fresh":
        if handle is not None or source_plan["fromRevision"] is not None or source_plan["edits"]:
            return failure()
    else:
        if base is None or base["provider"] != capture["provider"] or any(base["context"][key] != context[key] for key in catalog["parseKeyFields"]) or base["context"]["sourceRevision"] != source_plan["fromRevision"]:
            return failure()
        if mode in ("unchanged", "reuse-parse") and (base["source"] != source or source_plan["fromRevision"] != source_plan["toRevision"] or source_plan["edits"]):
            return failure()
    if (capture["mode"] == "query-only") != (mode == "reuse-parse"):
        return failure()
    old_facts = facts(base["forest"]) if base else None
    highlights = reference.queries()
    if context["highlightQueryGeneration"] == 2:
        highlights["javascript"] = "(identifier) @symbol"
    if mode == "reuse-parse":
        block = base["forest"]["block"]
    elif mode in ("incremental", "unchanged"):
        edited = base["forest"]["block"].copy()
        assert len(source_plan["edits"]) <= 64
        for edit in source_plan["edits"]:
            edit = dict(edit)
            for key in ("start_point", "old_end_point", "new_end_point"):
                edit[key] = (edit[key]["row"], edit[key]["column"])
            edited.edit(**edit)
        block = reference.ts.Parser(reference.LANGUAGES["markdown"]).parse(source, old_tree=edited)
    else:
        block = reference.ts.Parser(reference.LANGUAGES["markdown"]).parse(source)
    if block is None:
        return failure("exhausted")
    forest = {}
    row = reference.mixed({"source": list(source), "available": ["javascript"]}, highlights, block, forest, base["forest"] if mode == "reuse-parse" else None)
    if base:
        assert facts(base["forest"]) == old_facts, "retained published forest was mutated"
    if mode == "reuse-parse":
        assert set(forest) == set(base["forest"]), "query-only changed native forest membership"
        assert all(tree is base["forest"][key] for key, tree in forest.items()), "query-only reparsed instead of reusing exact native trees"
    else:
        fresh = reference.ts.Parser(reference.LANGUAGES["markdown"]).parse(source)
        assert reference.signature(block) == reference.signature(fresh), "bound incremental tree differs from fresh"
        fresh_row = reference.mixed({"source": list(source), "available": ["javascript"]}, highlights)
        assert row == fresh_row, "bound native observations differ from fresh"
    packet = {"context": context, "source": list(source), "parseState": "syntax-errors" if row["hostHasError"] else "parsed", "coverage": "partial" if row["partial"] else "complete-within-profile", "regions": row["regions"], "captures": row["resolved"]}
    issued += 1
    records[issued] = {"context": context, "provider": capture["provider"], "source": source, "forest": forest}
    return {"kind": "result", "handle": issued, "packet": packet, "facts": {"forestTrees": len(forest), "exactQueryReuse": mode == "reuse-parse", "basePreserved": base is not None, "inputMode": mode}}


def run(request):
    global messages, last_serial
    assert reference.info() == expected, "native tool or query fingerprint mismatch"
    messages += 1
    assert messages <= catalog["maxMessages"]
    serial = request["serial"]
    assert type(serial) is int and serial > last_serial
    last_serial = serial
    operation = request["operation"]
    assert operation in catalog["operations"]
    fields = {"operation", "serial"} | ({"capture", "base"} if operation == "execute" else {"handle"} if operation == "drop" else set())
    assert set(request) == fields
    if operation == "metadata":
        result = {"kind": "metadata", "metadata": expected, "profile": catalog["profile"]}
    elif operation == "execute":
        result = execute(request)
    elif operation == "drop":
        handle = request["handle"]
        if type(handle) is not int or handle not in records:
            result = {"kind": "unavailable"}
        else:
            del records[handle]
            assert handle not in records, "retired native handle remains reachable"
            result = {"kind": "dropped"}
    else:
        assert not records, "cannot close with retained native forest obligations"
        result = {"kind": "closed"}
    assert reference.info() == expected, "native tool changed during execution"
    return {"serial": serial, **result}


if __name__ == "__main__":
    try:
        while True:
            data = sys.stdin.buffer.readline(catalog["maxMessageBytes"] + 1)
            if not data:
                break
            assert len(data) <= catalog["maxMessageBytes"] and data.endswith(b"\n")
            request = json.loads(data)
            result = json.dumps(run(request), separators=(",", ":")).encode()
            assert len(result) + 1 <= catalog["maxMessageBytes"]
            sys.stdout.buffer.write(result + b"\n")
            sys.stdout.buffer.flush()
            if request["operation"] == "close":
                break
    except AssertionError as error:
        print("REFERENCE_ASSERTION:", str(error), file=sys.stderr)
        sys.exit(2)
