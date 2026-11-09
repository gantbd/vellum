"""Pinned native verification worker, not an editor runtime or hostile-code sandbox."""
import hashlib
import importlib.metadata as metadata
import importlib.resources as resources
import json
import pathlib
import platform
import sys

import tree_sitter as ts
import tree_sitter_markdown as markdown
import tree_sitter_javascript as javascript

DISTRIBUTIONS = ("tree-sitter", "tree-sitter-markdown", "tree-sitter-javascript")
LANGUAGES = {
    "markdown": ts.Language(markdown.language()),
    "markdown_inline": ts.Language(markdown.inline_language()),
    "javascript": ts.Language(javascript.language()),
}
ALIASES = {"js": "javascript", "javascript": "javascript"}
MAX_SOURCE = 32768
MAX_NODES = 8192
MAX_CAPTURES = 4096
MAX_REGIONS = 32
COMPILED = {}


def info():
    distributions = {}
    for name in DISTRIBUTIONS:
        dist = metadata.distribution(name)
        files = {}
        for file in dist.files:
            relative = str(file)
            if relative.endswith((".py", ".pyi", ".so", ".scm")) or relative.endswith("/LICENSE"):
                files[relative] = hashlib.sha256(dist.locate_file(file).read_bytes()).hexdigest()
        distributions[name] = {"version": dist.version, "files": files}
    return {
        "python": platform.python_version(), "platform": sys.platform, "machine": platform.machine(),
        "distributions": distributions,
        "languages": {name: {"abi": value.abi_version, "semanticVersion": list(value.semantic_version)} for name, value in LANGUAGES.items()},
    }


def source_bytes(value):
    assert isinstance(value, list) and len(value) <= MAX_SOURCE
    assert all(type(n) is int and 0 <= n <= 255 for n in value)
    value = bytes(value)
    value.decode("utf8")
    return value


def point(source, offset):
    return (source[:offset].count(b"\n"), offset - (source.rfind(b"\n", 0, offset) + 1))


def walk(node):
    stack = [node]
    count = 0
    while stack:
        current = stack.pop()
        count += 1
        assert count <= MAX_NODES
        yield current
        stack.extend(reversed(current.children))


def signature(tree):
    return [(node.type, node.is_named, node.is_missing, node.is_error,
             node.start_byte, node.end_byte, tuple(node.start_point), tuple(node.end_point),
             tuple(node.field_name_for_child(i) for i in range(node.child_count)))
            for node in walk(tree.root_node)]


def query(language, text, tree):
    key = (language.name, text)
    compiled = COMPILED.get(key)
    if compiled is None:
        compiled = ts.Query(language, text)
        # This lexical reference does not assert a locals/scope-resolution service.
        for pattern in range(compiled.pattern_count):
            if compiled.pattern_assertions(pattern):
                compiled.disable_pattern(pattern)
        COMPILED[key] = compiled
    cursor = ts.QueryCursor(compiled, match_limit=4096)
    captures = cursor.captures(tree.root_node)
    assert not cursor.did_exceed_match_limit
    result = sorted((name, node.start_byte, node.end_byte) for name, nodes in captures.items() for node in nodes)
    assert len(result) <= MAX_CAPTURES
    return result


def queries():
    return {
        "markdown": resources.files(markdown).joinpath("queries/markdown/highlights.scm").read_text(),
        "markdown_inline": resources.files(markdown).joinpath("queries/markdown_inline/highlights.scm").read_text(),
        "javascript": javascript.HIGHLIGHTS_QUERY,
    }


def incremental(case, highlights):
    language = LANGUAGES[case["language"]]
    before, after = source_bytes(case["before"]), source_bytes(case["source"])
    parser = ts.Parser(language)
    old = parser.parse(before)
    old_signature = signature(old)
    old_change_flags = [node.has_changes for node in walk(old.root_node)]
    old_captures = query(language, highlights[case["language"]], old)
    edited = old.copy()
    assert edited is not old, "editable tree must be a distinct private copy"
    assert len(case["edits"]) <= 64
    for edit in case["edits"]:
        edit = dict(edit)
        for key in ("start_point", "old_end_point", "new_end_point"):
            edit[key] = (edit[key]["row"], edit[key]["column"])
        edited.edit(**edit)
    updated = parser.parse(after, old_tree=edited) if case["mode"] != "fresh" else parser.parse(after)
    fresh = ts.Parser(language).parse(after)
    assert signature(updated) == signature(fresh), "incremental tree differs from fresh"
    assert query(language, highlights[case["language"]], updated) == query(language, highlights[case["language"]], fresh), "incremental captures differ"
    assert signature(old) == old_signature, "published tree was mutated"
    assert [node.has_changes for node in walk(old.root_node)] == old_change_flags, "published change flags were mutated"
    assert query(language, highlights[case["language"]], old) == old_captures, "published captures were mutated"
    for node in walk(updated.root_node):
        assert tuple(node.start_point) == point(after, node.start_byte)
        assert tuple(node.end_point) == point(after, node.end_byte)
    return {"equivalent": True, "publishedTreePreserved": True, "capturesConsistent": True,
            "hasError": updated.root_node.has_error, "steps": len(case["edits"]),
            "nodes": len(signature(updated)), "captures": len(query(language, highlights[case["language"]], updated)),
            "changedRanges": len(edited.changed_ranges(updated)), "capturesChanged": old_captures != query(language, highlights[case["language"]], updated)}


def subtract(interval, holes):
    pieces = [interval]
    for a, b in holes:
        next_pieces = []
        for lo, hi in pieces:
            if b <= lo or hi <= a:
                next_pieces.append((lo, hi))
            else:
                if lo < a:
                    next_pieces.append((lo, a))
                if b < hi:
                    next_pieces.append((b, hi))
        pieces = next_pieces
    return [(a, b) for a, b in pieces if a < b]


def intersections(lo, hi, ranges):
    return [(max(lo, a), min(hi, b)) for a, b in ranges if max(lo, a) < min(hi, b)]


def source_target(lo, hi, ranges):
    # No clipping or widening a proposed contiguous authored edit across host holes.
    return (lo, hi) if any(a <= lo <= hi <= b for a, b in ranges) else None


def included_parser(language, source, ranges):
    native_ranges = [ts.Range(point(source, a), point(source, b), a, b) for a, b in ranges]
    return ts.Parser(language, included_ranges=native_ranges).parse(source)


def resolve_display(display, length):
    priorities = {"host-block": 0, "host-inline": 1, "child": 2}
    boundaries = sorted({0, length} | {p for _, _, lo, hi in display for p in (lo, hi)})
    result = []
    for lo, hi in zip(boundaries, boundaries[1:]):
        candidates = [(layer, name, a, b) for layer, name, a, b in display if a <= lo < hi <= b]
        if candidates:
            selected = min(candidates, key=lambda row: (-priorities[row[0]], row[3] - row[2], row[1]))
            result.append((selected[0], selected[1], lo, hi))
    return result


def mixed(case, highlights, block=None, forest=None, reuse=None):
    source = source_bytes(case["source"])
    block = ts.Parser(LANGUAGES["markdown"]).parse(source) if block is None else block
    if forest is not None:
        forest["block"] = block

    def child_tree(language, ranges):
        key = (language, tuple(tuple(span) for span in ranges))
        if reuse is not None:
            assert key in reuse, "query-only forest does not match included ranges"
            tree = reuse[key]
        else:
            tree = included_parser(LANGUAGES[language], source, ranges)
        if forest is not None:
            forest[key] = tree
        return tree
    nodes = list(walk(block.root_node))
    fences = [node for node in nodes if node.type == "fenced_code_block"]
    assert len(fences) <= MAX_REGIONS
    regions = []
    child_budget = case.get("childBudget", 32)
    assert type(child_budget) is int and 0 <= child_budget <= 32
    child_count = 0
    partial = False
    host_captures = query(LANGUAGES["markdown"], highlights["markdown"], block)
    inline_captures = []
    # Follow the upstream split grammar: parse each inline node, excluding children.
    for inline in [n for n in nodes if n.type == "inline"]:
        holes = [(n.start_byte, n.end_byte) for n in inline.named_children if n.start_byte < n.end_byte]
        ranges = subtract((inline.start_byte, inline.end_byte), holes)
        if ranges:
            if child_count >= child_budget:
                partial = True
                continue
            child_count += 1
            tree = child_tree("markdown_inline", ranges)
            inline_captures.extend(query(LANGUAGES["markdown_inline"], highlights["markdown_inline"], tree))
    all_body_ranges = [(n.start_byte, n.end_byte) for n in nodes if n.type == "code_fence_content" and n.start_byte < n.end_byte]
    for fence in fences:
        bodies = [n for n in fence.named_children if n.type == "code_fence_content"]
        labels = [n for n in walk(fence) if n.type == "language"]
        label = labels[0].text.decode("utf8") if labels else ""
        language = ALIASES.get(label.lower() if label.isascii() else label)
        body = bodies[0] if bodies else None
        delimiters = [n for n in fence.children if n.type == "fenced_code_block_delimiter"]
        closed = len(delimiters) == 2
        region = {"label": label, "language": language, "closed": closed, "body": [body.start_byte, body.end_byte] if body else None,
                  "kind": "empty-body" if language and closed else "incomplete-fence" if not closed else "unsupported-language", "ranges": [], "captures": [], "hasError": False}
        if body:
            holes = [(n.start_byte, n.end_byte) for n in walk(body) if n.type == "block_continuation" and n.start_byte < n.end_byte]
            ranges = subtract((body.start_byte, body.end_byte), holes)
            region["ranges"] = ranges
            if not closed:
                region["kind"] = "incomplete-fence"
            elif language and language not in case["available"]:
                region["kind"] = "grammar-unavailable"
            elif language and ranges and child_count >= child_budget:
                region["kind"] = "region-budget"
                partial = True
            elif language and ranges:
                child_ranges = ranges
                child_count += 1
                tree = child_tree(language, child_ranges)
                assert body.start_byte <= tree.root_node.start_byte <= tree.root_node.end_byte <= body.end_byte, "child escaped its fence"
                captures = query(LANGUAGES[language], highlights[language], tree)
                projected = [(name, a, b) for name, lo, hi in captures for a, b in intersections(lo, hi, ranges)]
                assert all(body.start_byte <= a < b <= body.end_byte for _, a, b in projected)
                assert all(not (a < y and x < b) for _, a, b in projected for x, y in holes)
                for lo, hi in holes:
                    assert source_target(max(body.start_byte, lo - 1), min(body.end_byte, hi + 1), ranges) is None
                region.update(kind="parsed", captures=projected, hasError=tree.root_node.has_error)
        regions.append(region)
    supported_bodies = [tuple(span) for r in regions if r["kind"] == "parsed" for span in r["ranges"]]
    # Child-owned body interpretation overrides host literal fallback only there.
    display = [("host-block", "text.literal" if name == "none" else name, a, b) for name, lo, hi in host_captures for a, b in subtract((lo, hi), supported_bodies)]
    display.extend(("host-inline", name, lo, hi) for name, lo, hi in inline_captures)
    display.extend(("child", name, lo, hi) for r in regions for name, lo, hi in r["captures"])
    assert len(display) <= MAX_CAPTURES
    return {"regions": regions, "hostCaptures": host_captures, "inlineCaptures": inline_captures, "display": display,
            "resolved": resolve_display(display, len(source)),
            "hostHasError": block.root_node.has_error, "partial": partial, "childAnalyses": child_count}


def run(request):
    if request["operation"] == "metadata":
        return info()
    assert request["operation"] == "run"
    expected = json.loads(pathlib.Path(__file__).with_name("tree-sitter-reference.json").read_text())["environment"]
    assert info() == expected, "native tool or query fingerprint mismatch"
    assert len(request["cases"]) <= 1024 and len(request["mixed"]) <= 128
    highlights = queries()
    rows = [incremental(case, highlights) for case in request["cases"]]
    mixed_rows = [mixed(case, highlights) for case in request["mixed"]]
    sample = ts.Parser(LANGUAGES["javascript"]).parse(b"const name = 1;")
    before = signature(sample)
    old_highlights = query(LANGUAGES["javascript"], highlights["javascript"], sample)
    rejected = False
    try:
        ts.Query(LANGUAGES["javascript"], "(not_a_real_node_type) @invalid")
    except ts.QueryError:
        rejected = True
    assert rejected and signature(sample) == before
    assert old_highlights == query(LANGUAGES["javascript"], highlights["javascript"], sample)
    other = query(LANGUAGES["javascript"], "(identifier) @symbol", sample)
    assert other != old_highlights and signature(sample) == before
    ties = resolve_display([("host-block", "host", 0, 4), ("host-inline", "inline", 0, 4),
                            ("child", "wide", 0, 4), ("child", "z", 1, 2), ("child", "a", 1, 2)], 4)
    assert ties == [("child", "wide", 0, 1), ("child", "a", 1, 2), ("child", "wide", 2, 4)]
    assert info() == expected, "native tool changed during execution"
    js_query = ts.Query(LANGUAGES["javascript"], highlights["javascript"])
    omitted = sum(bool(js_query.pattern_assertions(i)) for i in range(js_query.pattern_count))
    return {"metadata": expected, "rows": rows, "mixed": mixed_rows, "omittedPropertyPatterns": omitted,
            "nativeControls": {"invalidQueryPreservesPrior": True, "queryOnlyPreservesTree": True, "displayTiesChecked": True}}


if __name__ == "__main__":
    try:
        data = sys.stdin.buffer.read(8 * 1024 * 1024 + 1)
        assert len(data) <= 8 * 1024 * 1024
        result = json.dumps(run(json.loads(data)), separators=(",", ":")).encode()
        assert len(result) <= 8 * 1024 * 1024
        sys.stdout.buffer.write(result)
    except AssertionError as error:
        print("REFERENCE_ASSERTION:", str(error), file=sys.stderr)
        sys.exit(2)
