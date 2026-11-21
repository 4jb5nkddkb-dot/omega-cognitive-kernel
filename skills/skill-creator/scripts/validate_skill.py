#!/usr/bin/env python3
"""Validate repository Agent Skills without third-party dependencies."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path, PurePosixPath

NAME_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
FRONTMATTER_RE = re.compile(r"\A---[ \t]*\n(.*?)\n---[ \t]*(?:\n|\Z)", re.DOTALL)
FIELD_RE = re.compile(r"^([A-Za-z][A-Za-z0-9_-]*):(?:[ \t]*(.*))?$")
LINK_RE = re.compile(r"(?<!!)\[[^\]\n]*\]\(\s*([^)\s]+)(?:\s+[^)]*)?\)")
FENCE_RE = re.compile(r"^\s*(```|~~~)")
ALLOWED_FIELDS = {
    "name", "description", "license", "compatibility", "metadata", "allowed-tools"
}
SECRET_RES = (
    re.compile(r"-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----"),
    re.compile(r"\b(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,})\b"),
    re.compile(r"\b(?:sk-[A-Za-z0-9_-]{20,}|AKIA[0-9A-Z]{16}|xox[baprs]-[A-Za-z0-9-]{20,})\b"),
    re.compile(
        r"(?i)\b(?:api[_-]?key|access[_-]?token|auth[_-]?token|password|client[_-]?secret)"
        r"\b\s*[:=]\s*[\"']?(?!<|\$\{?|\{\{|your[_-]|example|placeholder|redacted)[^\s\"']{8,}"
    ),
)


class YamlError(ValueError):
    """The frontmatter is outside the supported portable YAML subset."""


def _scalar(value: str) -> str:
    value = value.strip()
    if not value:
        raise YamlError("empty scalar; use \"\" for an intentional empty value")
    if value[0] in "\"'":
        if len(value) < 2 or value[-1] != value[0]:
            raise YamlError("unterminated quoted scalar")
        quote = value[0]
        inner = value[1:-1]
        if quote == "'":
            return inner.replace("''", "'")
        try:
            result = json.loads(value)
        except json.JSONDecodeError as exc:
            raise YamlError(f"invalid double-quoted scalar: {exc.msg}") from exc
        if not isinstance(result, str):
            raise YamlError("expected a string scalar")
        return result
    if value[0] in "[{&*!|>":
        raise YamlError("flow collections, anchors, tags, and block scalars are not supported")
    if " #" in value or "\t#" in value:
        value = re.split(r"[ \t]+#", value, maxsplit=1)[0].rstrip()
    return value


def parse_frontmatter(text: str) -> dict[str, object]:
    """Parse string fields and a string-to-string metadata mapping."""
    match = FRONTMATTER_RE.match(text)
    if not match:
        raise YamlError("SKILL.md must start with YAML frontmatter delimited by --- lines")
    lines = match.group(1).splitlines()
    result: dict[str, object] = {}
    index = 0
    while index < len(lines):
        line = lines[index]
        if not line.strip() or line.lstrip().startswith("#"):
            index += 1
            continue
        if line[0].isspace():
            raise YamlError(f"unexpected indentation on frontmatter line {index + 1}")
        field = FIELD_RE.match(line)
        if not field:
            raise YamlError(f"expected 'field: value' on frontmatter line {index + 1}")
        key, raw = field.groups()
        if key in result:
            raise YamlError(f"duplicate frontmatter field: {key}")
        index += 1
        if raw and raw.strip():
            result[key] = _scalar(raw)
            continue
        if key != "metadata":
            raise YamlError(f"{key} must have a string value")
        metadata: dict[str, str] = {}
        while index < len(lines):
            child = lines[index]
            if not child.strip() or child.lstrip().startswith("#"):
                index += 1
                continue
            if not child.startswith((" ", "\t")):
                break
            if not child.startswith("  ") or child.startswith("   ") or child[2].isspace():
                raise YamlError("metadata entries must be indented by exactly two spaces")
            entry = FIELD_RE.match(child.strip())
            if not entry:
                raise YamlError(f"invalid metadata entry on frontmatter line {index + 1}")
            child_key, child_raw = entry.groups()
            if child_key in metadata:
                raise YamlError(f"duplicate metadata key: {child_key}")
            if not child_raw:
                raise YamlError(f"metadata.{child_key} must have a string value")
            metadata[child_key] = _scalar(child_raw)
            index += 1
        result[key] = metadata
    return result


def _unquote_link(target: str) -> str:
    if target.startswith("<") and target.endswith(">"):
        return target[1:-1]
    return target


def _check_links(skill_dir: Path, markdown_files: list[Path]) -> list[str]:
    errors: list[str] = []
    for source in markdown_files:
        text = source.read_text(encoding="utf-8")
        fenced = False
        fence_marker = ""
        for line_number, line in enumerate(text.splitlines(), 1):
            fence = FENCE_RE.match(line)
            if fence:
                marker = fence.group(1)
                if not fenced:
                    fenced, fence_marker = True, marker
                elif marker[0] == fence_marker[0]:
                    fenced = False
                continue
            if fenced:
                continue
            for match in LINK_RE.finditer(line):
                target = _unquote_link(match.group(1))
                if not target or target.startswith(("#", "http://", "https://", "mailto:")):
                    continue
                if re.match(r"^[A-Za-z][A-Za-z0-9+.-]*:", target):
                    continue
                path_text = target.split("#", 1)[0].split("?", 1)[0]
                if not path_text:
                    continue
                if PurePosixPath(path_text).is_absolute() or re.match(r"^[A-Za-z]:[/\\]", path_text):
                    errors.append(f"{source.name}:{line_number}: local link must be relative: {target}")
                    continue
                resolved = (source.parent / path_text).resolve()
                try:
                    resolved.relative_to(skill_dir.resolve())
                except ValueError:
                    errors.append(f"{source.name}:{line_number}: link leaves skill directory: {target}")
                    continue
                if not resolved.exists():
                    errors.append(f"{source.name}:{line_number}: missing linked resource: {target}")
    return errors


def _check_secrets(skill_dir: Path, paths: list[Path]) -> list[str]:
    errors: list[str] = []
    # Do not flag the detector's own patterns or its intentional test fixtures.
    if (skill_dir / "scripts" / "validate_skill.py").is_file() and (
        skill_dir / "tests" / "test_validate_skill.py"
    ).is_file():
        paths = [path for path in paths if path.suffix != ".py"]
    for path in paths:
        if path.suffix.lower() not in {".md", ".py", ".sh", ".toml", ".yaml", ".yml", ".json", ".txt"}:
            continue
        for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            if any(pattern.search(line) for pattern in SECRET_RES):
                errors.append(f"{path.relative_to(skill_dir)}:{line_number}: possible credential; remove it")
    return errors


def validate_skill(skill_dir: Path) -> list[str]:
    """Return all validation errors for one skill directory."""
    skill_dir = skill_dir.resolve()
    if not skill_dir.is_dir():
        return [f"{skill_dir}: not a directory"]
    skill_file = skill_dir / "SKILL.md"
    if not skill_file.is_file():
        return [f"{skill_dir}: SKILL.md not found"]
    try:
        text = skill_file.read_text(encoding="utf-8")
        frontmatter = parse_frontmatter(text)
    except (UnicodeError, OSError, YamlError) as exc:
        return [f"{skill_dir}: {exc}"]
    errors: list[str] = []
    unknown = set(frontmatter) - ALLOWED_FIELDS
    if unknown:
        errors.append(f"unknown frontmatter field(s): {', '.join(sorted(unknown))}")
    for required in ("name", "description"):
        if required not in frontmatter:
            errors.append(f"missing required frontmatter field: {required}")
    name = frontmatter.get("name")
    if isinstance(name, str):
        if not 1 <= len(name) <= 64:
            errors.append("name must contain 1-64 characters")
        if not NAME_RE.fullmatch(name):
            errors.append("name must use lowercase letters, digits, and single hyphens")
        if name != skill_dir.name:
            errors.append(f"name {name!r} does not match directory {skill_dir.name!r}")
    description = frontmatter.get("description")
    if isinstance(description, str):
        if not description.strip() or len(description) > 1024:
            errors.append("description must contain 1-1024 non-whitespace characters")
        if "<" in description or ">" in description:
            errors.append("description cannot contain angle brackets")
    for field in ("license", "compatibility", "allowed-tools"):
        if field in frontmatter and not isinstance(frontmatter[field], str):
            errors.append(f"{field} must be a string")
    compatibility = frontmatter.get("compatibility")
    if isinstance(compatibility, str) and not 1 <= len(compatibility.strip()) <= 500:
        errors.append("compatibility must contain 1-500 non-whitespace characters")
    metadata = frontmatter.get("metadata")
    if metadata is not None and not isinstance(metadata, dict):
        errors.append("metadata must be a mapping of string keys to string values")
    match = FRONTMATTER_RE.match(text)
    if match and not text[match.end():].strip():
        errors.append("SKILL.md must contain Markdown instructions after frontmatter")

    files = sorted(path for path in skill_dir.rglob("*") if path.is_file() or path.is_symlink())
    for path in files:
        if path.is_symlink():
            try:
                path.resolve().relative_to(skill_dir)
            except ValueError:
                errors.append(f"unsafe symlink outside skill: {path.relative_to(skill_dir)}")
        if path.name.endswith((".bak", ".swp", ".pyc")) or path.name == ".clinerules":
            errors.append(f"local or backup file must not be bundled: {path.relative_to(skill_dir)}")
    regular_files = [path for path in files if path.is_file()]
    markdown_files = [path for path in regular_files if path.suffix.lower() == ".md"]
    errors.extend(_check_links(skill_dir, markdown_files))
    errors.extend(_check_secrets(skill_dir, regular_files))
    return errors



def skill_directories(path: Path) -> tuple[list[Path], list[str]]:
    if (path / "SKILL.md").is_file():
        return [path], []
    if not path.is_dir():
        return [], [f"{path}: not a skill or skill-pack directory"]
    children = sorted(child for child in path.iterdir() if child.is_dir() and not child.name.startswith("."))
    if not children:
        return [], [f"{path}: no skill directories found"]
    return children, []


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path", type=Path, help="skill directory or parent directory of skills")
    args = parser.parse_args(argv)
    directories, errors = skill_directories(args.path)
    failed = 0
    for directory in directories:
        problems = validate_skill(directory)
        if problems:
            failed += 1
            for problem in problems:
                print(f"FAIL {directory}: {problem}", file=sys.stderr)
        else:
            print(f"PASS {directory}")
    for error in errors:
        print(f"FAIL {error}", file=sys.stderr)
    if directories and not errors:
        print(f"Validated {len(directories)} skill(s): {len(directories) - failed} passed, {failed} failed.")
    return 1 if failed or errors else 0


if __name__ == "__main__":
    sys.exit(main())

