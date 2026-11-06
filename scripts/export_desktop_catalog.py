#!/usr/bin/env python3
"""Export the pinned Swift registry's declarative literals for the Rust host.

No Swift execution, network access or guessed version bounds. Reject unsupported
syntax instead of silently dropping fields. `--check` validates committed output.
"""
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "apps/ios/Sources/Core/GoldenNuggetPort/TweakCatalog.swift"
OUTPUT = ROOT / "crates/customization/catalog.json"


def split_top(text, separator=","):
    parts, start, stack, quoted, escaped = [], 0, [], False, False
    for index, char in enumerate(text):
        if quoted:
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                quoted = False
            continue
        if char == '"':
            quoted = True
        elif char in "([{":
            stack.append(char)
        elif char in ")]}":
            if not stack or stack.pop() != {")": "(", "]": "[", "}": "{"}[char]:
                raise ValueError("Unbalanced Swift literal")
        elif char == separator and not stack:
            parts.append(text[start:index].strip())
            start = index + 1
    if quoted or stack:
        raise ValueError("Unterminated Swift literal")
    parts.append(text[start:].strip())
    return [part for part in parts if part]


def scalar(text):
    match = re.fullmatch(r"\.(bool|int|double|string)\((.*)\)", text, re.S)
    if not match:
        raise ValueError("Unsupported Swift scalar: " + text[:80])
    kind, raw = match.groups()
    value = json.loads(raw)
    if kind == "bool" and type(value) is not bool:
        raise ValueError("Invalid bool")
    if kind == "int" and type(value) is not int:
        raise ValueError("Invalid int")
    if kind == "double" and type(value) not in (int, float):
        raise ValueError("Invalid double")
    if kind == "string" and not isinstance(value, str):
        raise ValueError("Invalid string")
    return {"type": kind, "value": value}


def fields(text):
    result = {}
    for field in split_top(text):
        key, value = field.split(":", 1)
        key = key.strip()
        if key in result:
            raise ValueError("Duplicate Swift field")
        result[key] = value.strip()
    return result


def records(source):
    start = source.index("static let all: [TweakSpec] = [")
    end = source.index("static let allWithDaemons:", start)
    text = source[start:end]
    pos = 0
    while True:
        pos = text.find("TweakSpec(", pos)
        if pos < 0:
            return
        begin = pos + len("TweakSpec(")
        depth, quoted, escaped = 1, False, False
        for index in range(begin, len(text)):
            char = text[index]
            if quoted:
                if escaped: escaped = False
                elif char == "\\": escaped = True
                elif char == '"': quoted = False
            elif char == '"': quoted = True
            elif char == "(": depth += 1
            elif char == ")":
                depth -= 1
                if depth == 0:
                    yield fields(text[begin:index])
                    pos = index + 1
                    break
        else:
            raise ValueError("Unclosed TweakSpec")


def build():
    source = SOURCE.read_text()
    provenance = json.loads((ROOT / "LICENSES/GoldenNuggetMobile-import.json").read_text())
    digest = hashlib.sha256(SOURCE.read_bytes()).hexdigest()
    if digest != provenance["outputs"][SOURCE.relative_to(ROOT).as_posix()]:
        raise ValueError("Swift snapshot does not match the pinned import")
    locations = dict(re.findall(r'case (\w+) = "([^"]+)"', source[:source.index("enum TweakSection")]))
    specs = []
    for item in records(source):
        result = {}
        for name, raw in item.items():
            if name == "location":
                result["path"] = locations[raw.removeprefix(".")]
            elif name in ("section", "kind"):
                result[name] = raw.removeprefix(".")
            elif name in ("value", "offValue"):
                result[name] = None if raw == "nil" else scalar(raw)
            elif name == "multiValues":
                if raw == "nil":
                    result[name] = None
                else:
                    result[name] = {json.loads(k.strip()): scalar(v.strip()) for k, v in
                                    (part.split(":", 1) for part in split_top(raw[1:-1]))}
            else:
                result[name] = None if raw == "nil" else json.loads(raw)
        specs.append(result)
    if len(specs) != 258 or len({spec["id"] for spec in specs}) != len(specs):
        raise ValueError("Unexpected or duplicate registry entries")
    return {"repository": provenance["repository"], "revision": provenance["revision"],
            "swift_sha256": digest, "specs": specs}


if __name__ == "__main__":
    content = json.dumps(build(), ensure_ascii=False, indent=2, allow_nan=False) + "\n"
    if "--check" in sys.argv:
        if not OUTPUT.is_file() or OUTPUT.read_text() != content:
            raise SystemExit("Desktop catalog is stale; run scripts/export_desktop_catalog.py")
        print("Desktop catalog matches the pinned Swift source")
    else:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_text(content)
        print("Exported 258 registry entries for Desktop")
