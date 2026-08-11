#!/usr/bin/env python3
"""Fail closed when the release workflow can bypass the normal CI contract.

The GitHub runner does not install a YAML library explicitly.  This checker
therefore parses only the small indentation-based subset used by our workflow
contracts and rejects structural syntax it does not understand.  It is not a
general YAML parser.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re
import sys
from typing import Iterable, Sequence


ROOT = Path(__file__).resolve().parent.parent
CI_PATH = ROOT / ".github" / "workflows" / "ci.yml"
RELEASE_PATH = ROOT / ".github" / "workflows" / "release.yml"

REUSABLE_CI = "./.github/workflows/ci.yml"
QUALITY_JOB = "quality-gates"
PUBLISH_GUARD = "github.event_name == 'push' && startsWith(github.ref, 'refs/tags/')"

_FIELD = re.compile(r"^([A-Za-z_][A-Za-z0-9_-]*):(?:[ ]*(.*))?$")
_BLOCK_SCALARS = {"|", "|-", "|+", ">", ">-", ">+"}


class WorkflowParseError(ValueError):
    """The workflow left the deliberately supported YAML subset."""


@dataclass(frozen=True)
class SourceLine:
    number: int
    text: str

    @property
    def indent(self) -> int:
        return len(self.text) - len(self.text.lstrip(" "))

    @property
    def content(self) -> str:
        return self.text[self.indent :]

    @property
    def ignored(self) -> bool:
        stripped = self.text.strip()
        return not stripped or stripped.startswith("#")


@dataclass
class Field:
    key: str
    raw_value: str
    indent: int
    line: int
    children: list[SourceLine]


@dataclass
class Step:
    fields: dict[str, Field]
    line: int


@dataclass(frozen=True)
class ProductContract:
    tag_prefix: str
    version_command: str
    release_notes_command: str
    build_step: str
    build_command: str
    upload_name: str
    upload_paths: str
    make_latest: str
    publish_files: str
    publish_body: str
    appcast_command: str | None = None
    feed_command: str | None = None


def _app_version_command(app_dir: str, tag_prefix: str) -> str:
    return "\n".join(
        (
            f"$pubspec = Get-Content {app_dir}/pubspec.yaml -Raw",
            r"if ($pubspec -notmatch '(?m)^version:\s*([0-9]+\.[0-9]+\.[0-9]+)\s*$') {",
            "  Write-Error 'pubspec.yaml version must be plain X.Y.Z (no build number)'",
            "  exit 1",
            "}",
            "$version = $Matches[1]",
            "if ('${{ github.ref_type }}' -eq 'tag') {",
            "  $tag = $env:GITHUB_REF_NAME",
            f'  if ($tag -ne "{tag_prefix}$version") {{',
            '    Write-Error "tag $tag does not match pubspec version $version"',
            "    exit 1",
            "  }",
            "}",
            '"version=$version" >> $env:GITHUB_OUTPUT',
        )
    )


def _cli_version_command() -> str:
    return "\n".join(
        (
            "$manifest = Get-Content crates/gore/Cargo.toml -Raw",
            r'''if ($manifest -notmatch '(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"') {''',
            "  Write-Error 'Cargo.toml version must be X.Y.Z'",
            "  exit 1",
            "}",
            "$version = $Matches[1]",
            "if ('${{ github.ref_type }}' -eq 'tag') {",
            "  $tag = $env:GITHUB_REF_NAME",
            '  if ($tag -ne "gore-cli-v$version") {',
            '    Write-Error "tag $tag does not match Cargo.toml version $version"',
            "    exit 1",
            "  }",
            "}",
            '"version=$version" >> $env:GITHUB_OUTPUT',
        )
    )


def _release_notes_command(changelog: str, output_dir: str) -> str:
    return "\n".join(
        (
            "$version = '${{ steps.version.outputs.version }}'",
            "$isTag = '${{ github.ref_type }}' -eq 'tag'",
            f"$changelog = Get-Content {changelog} -Raw",
            r'''$pattern = "(?ms)^## \[$([regex]::Escape($version))\][^`r`n]*`r?`n(.*?)(?=^## \[|\z)"''',
            "$notes = ''",
            "if ($changelog -match $pattern) { $notes = $Matches[1].Trim() }",
            "if (-not $notes) {",
            '  if ($isTag) { Write-Error "CHANGELOG.md has no non-empty section for $version"; exit 1 }',
            '  $notes = "Development build $version"',
            "}",
            f"New-Item -ItemType Directory -Force {output_dir} | Out-Null",
            f"[IO.File]::WriteAllText((Join-Path $PWD '{output_dir}/RELEASE_NOTES.md'), $notes)",
        )
    )


PRODUCTS: dict[str, ProductContract] = {
    "gore-save-editor": ProductContract(
        tag_prefix="gore-save-editor-v",
        version_command=_app_version_command(
            "apps/save-editor", "gore-save-editor-v"
        ),
        release_notes_command=_release_notes_command(
            "apps/save-editor/CHANGELOG.md", "dist/gore-save-editor"
        ),
        build_step="Build installer",
        build_command="python build.py gore-save-editor installer",
        upload_name="gore-save-editor-windows-x64",
        upload_paths=(
            "dist/gore-save-editor/*.zip\n"
            "dist/gore-save-editor/*.exe\n"
            "dist/gore-save-editor/appcast-windows.xml"
        ),
        make_latest='"true"',
        publish_files=(
            "dist/gore-save-editor/*.zip\n"
            "dist/gore-save-editor/*.exe\n"
            "dist/gore-save-editor/appcast-windows.xml"
        ),
        publish_body="dist/gore-save-editor/RELEASE_NOTES.md",
        appcast_command=(
            "python scripts/appcast.py --title gore-save-editor "
            "--version ${{ steps.version.outputs.version }} "
            "--installer dist/gore-save-editor/"
            "gore-save-editor-${{ steps.version.outputs.version }}-setup.exe "
            "--notes dist/gore-save-editor/RELEASE_NOTES.md "
            "--release-tag gore-save-editor-v${{ steps.version.outputs.version }} "
            "--output dist/gore-save-editor/appcast-windows.xml"
        ),
        feed_command=(
            "gh release view gore-save-editor-appcast 2>$null\n"
            "if ($LASTEXITCODE -ne 0) {\n"
            "  gh release create gore-save-editor-appcast `\n"
            '    --title "gore-save-editor update feed" `\n'
            '    --notes "Stable WinSparkle feed for gore-save-editor. Assets overwritten each release." `\n'
            "    --latest=false\n"
            "}\n"
            "gh release upload gore-save-editor-appcast `\n"
            "  dist/gore-save-editor/appcast-windows.xml --clobber"
        ),
    ),
    "gore-mod-studio": ProductContract(
        tag_prefix="gore-mod-studio-v",
        version_command=_app_version_command(
            "apps/mod-studio", "gore-mod-studio-v"
        ),
        release_notes_command=_release_notes_command(
            "apps/mod-studio/CHANGELOG.md", "dist/gore-mod-studio"
        ),
        build_step="Build installer",
        build_command="python build.py gore-mod-studio installer",
        upload_name="gore-mod-studio-windows-x64",
        upload_paths=(
            "dist/gore-mod-studio/*.zip\n"
            "dist/gore-mod-studio/*.exe\n"
            "dist/gore-mod-studio/appcast-windows.xml"
        ),
        make_latest='"false"',
        publish_files=(
            "dist/gore-mod-studio/*.zip\n"
            "dist/gore-mod-studio/gore-mod-studio-"
            "${{ steps.version.outputs.version }}-setup.exe"
        ),
        publish_body="dist/gore-mod-studio/RELEASE_NOTES.md",
        appcast_command=(
            "python scripts/appcast.py --title gore-mod-studio "
            "--version ${{ steps.version.outputs.version }} "
            "--installer dist/gore-mod-studio/"
            "gore-mod-studio-${{ steps.version.outputs.version }}-setup.exe "
            "--notes dist/gore-mod-studio/RELEASE_NOTES.md "
            "--release-tag gore-mod-studio-v${{ steps.version.outputs.version }} "
            "--output dist/gore-mod-studio/appcast-windows.xml"
        ),
        feed_command=(
            "gh release view gore-mod-studio-appcast 2>$null\n"
            "if ($LASTEXITCODE -ne 0) {\n"
            "  gh release create gore-mod-studio-appcast `\n"
            '    --title "gore-mod-studio update feed" `\n'
            '    --notes "Stable WinSparkle feed for gore-mod-studio. Assets overwritten each release." `\n'
            "    --latest=false\n"
            "}\n"
            "gh release upload gore-mod-studio-appcast `\n"
            "  dist/gore-mod-studio/appcast-windows.xml --clobber"
        ),
    ),
    "gore-mod-manager": ProductContract(
        tag_prefix="gore-mod-manager-v",
        version_command=_app_version_command(
            "apps/mod-manager", "gore-mod-manager-v"
        ),
        release_notes_command=_release_notes_command(
            "apps/mod-manager/CHANGELOG.md", "dist/gore-mod-manager"
        ),
        build_step="Build installer",
        build_command="python build.py gore-mod-manager installer",
        upload_name="gore-mod-manager-windows-x64",
        upload_paths=(
            "dist/gore-mod-manager/*.zip\n"
            "dist/gore-mod-manager/*.exe\n"
            "dist/gore-mod-manager/appcast-windows.xml"
        ),
        make_latest='"false"',
        publish_files=(
            "dist/gore-mod-manager/*.zip\n"
            "dist/gore-mod-manager/gore-mod-manager-"
            "${{ steps.version.outputs.version }}-setup.exe"
        ),
        publish_body="dist/gore-mod-manager/RELEASE_NOTES.md",
        appcast_command=(
            "python scripts/appcast.py --title gore-mod-manager "
            "--version ${{ steps.version.outputs.version }} "
            "--installer dist/gore-mod-manager/"
            "gore-mod-manager-${{ steps.version.outputs.version }}-setup.exe "
            "--notes dist/gore-mod-manager/RELEASE_NOTES.md "
            "--release-tag gore-mod-manager-v${{ steps.version.outputs.version }} "
            "--output dist/gore-mod-manager/appcast-windows.xml"
        ),
        feed_command=(
            "gh release view gore-mod-manager-appcast 2>$null\n"
            "if ($LASTEXITCODE -ne 0) {\n"
            "  gh release create gore-mod-manager-appcast `\n"
            '    --title "gore-mod-manager update feed" `\n'
            '    --notes "Stable WinSparkle feed for gore-mod-manager. Assets overwritten each release." `\n'
            "    --latest=false\n"
            "}\n"
            "gh release upload gore-mod-manager-appcast `\n"
            "  dist/gore-mod-manager/appcast-windows.xml --clobber"
        ),
    ),
    "gore-cli": ProductContract(
        tag_prefix="gore-cli-v",
        version_command=_cli_version_command(),
        release_notes_command=_release_notes_command(
            "crates/gore/CHANGELOG.md", "dist/gore-cli"
        ),
        build_step="Build distribution",
        build_command="python build.py gore-cli dist",
        upload_name="gore-cli-windows-x64",
        upload_paths="dist/gore-cli/*.zip",
        make_latest='"false"',
        publish_files="dist/gore-cli/*.zip",
        publish_body="dist/gore-cli/RELEASE_NOTES.md",
    ),
}


def _source(text: str, label: str) -> list[SourceLine]:
    lines: list[SourceLine] = []
    for number, raw in enumerate(text.splitlines(), start=1):
        if "\t" in raw:
            raise WorkflowParseError(f"{label}:{number}: tabs are not accepted")
        lines.append(SourceLine(number, raw.rstrip()))
    return lines


def _parse_fields(
    lines: Sequence[SourceLine], indent: int, context: str
) -> dict[str, Field]:
    fields: dict[str, Field] = {}
    current: Field | None = None
    for line in lines:
        if line.ignored:
            continue
        if line.indent < indent:
            raise WorkflowParseError(
                f"{context}:{line.number}: indentation escaped its parent"
            )
        if line.indent == indent:
            match = _FIELD.fullmatch(line.content)
            if match is None:
                raise WorkflowParseError(
                    f"{context}:{line.number}: unsupported mapping syntax"
                )
            key, raw_value = match.group(1), match.group(2) or ""
            if key in fields:
                raise WorkflowParseError(
                    f"{context}:{line.number}: duplicate key {key!r}"
                )
            current = Field(key, raw_value, indent, line.number, [])
            fields[key] = current
        elif current is None:
            raise WorkflowParseError(
                f"{context}:{line.number}: nested value has no owning key"
            )
        else:
            current.children.append(line)
    return fields


def _mapping(field: Field, context: str) -> dict[str, Field]:
    if field.raw_value:
        raise WorkflowParseError(
            f"{context}:{field.line}: expected an indented mapping"
        )
    return _parse_fields(field.children, field.indent + 2, context)


def _scalar(field: Field, context: str) -> str:
    raw = field.raw_value.strip()
    if raw in _BLOCK_SCALARS:
        content = [line for line in field.children if line.text.strip()]
        if not content:
            return ""
        minimum = min(line.indent for line in content)
        parts = [line.text[minimum:].rstrip() for line in content]
        if raw.startswith(">"):
            return " ".join(part.strip() for part in parts)
        return "\n".join(parts)
    if field.children:
        raise WorkflowParseError(
            f"{context}:{field.line}: scalar has unexpected nested content"
        )
    return raw


def _sequence(field: Field, context: str) -> list[str]:
    if field.raw_value:
        raise WorkflowParseError(
            f"{context}:{field.line}: expected an indented sequence"
        )
    expected_indent = field.indent + 2
    values: list[str] = []
    for line in field.children:
        if line.ignored:
            continue
        if line.indent != expected_indent or not line.content.startswith("- "):
            raise WorkflowParseError(
                f"{context}:{line.number}: unsupported sequence syntax"
            )
        value = line.content[2:].strip()
        if not value:
            raise WorkflowParseError(f"{context}:{line.number}: empty sequence item")
        values.append(value)
    return values


def _parse_steps(field: Field, context: str) -> list[Step]:
    if field.raw_value:
        raise WorkflowParseError(f"{context}:{field.line}: steps must be a list")
    item_indent = field.indent + 2
    chunks: list[list[SourceLine]] = []
    current: list[SourceLine] | None = None
    for line in field.children:
        if line.ignored:
            continue
        if line.indent < item_indent:
            raise WorkflowParseError(
                f"{context}:{line.number}: step indentation escaped its list"
            )
        if line.indent == item_indent:
            if not line.content.startswith("- "):
                raise WorkflowParseError(
                    f"{context}:{line.number}: unsupported step-list syntax"
                )
            current = [line]
            chunks.append(current)
        elif current is None:
            raise WorkflowParseError(
                f"{context}:{line.number}: step content has no list item"
            )
        else:
            current.append(line)

    steps: list[Step] = []
    for chunk in chunks:
        first = chunk[0]
        match = _FIELD.fullmatch(first.content[2:])
        if match is None:
            raise WorkflowParseError(
                f"{context}:{first.number}: every step must start with a field"
            )
        first_field = Field(
            match.group(1), match.group(2) or "", item_indent + 2, first.number, []
        )
        fields = {first_field.key: first_field}
        current_field = first_field
        for line in chunk[1:]:
            if line.ignored:
                continue
            direct_indent = item_indent + 2
            if line.indent < direct_indent:
                raise WorkflowParseError(
                    f"{context}:{line.number}: malformed step indentation"
                )
            if line.indent == direct_indent:
                match = _FIELD.fullmatch(line.content)
                if match is None:
                    raise WorkflowParseError(
                        f"{context}:{line.number}: unsupported step field"
                    )
                key = match.group(1)
                if key in fields:
                    raise WorkflowParseError(
                        f"{context}:{line.number}: duplicate step key {key!r}"
                    )
                current_field = Field(
                    key, match.group(2) or "", direct_indent, line.number, []
                )
                fields[key] = current_field
            else:
                current_field.children.append(line)
        steps.append(Step(fields, first.number))
    return steps


def _normalise_space(value: str) -> str:
    return " ".join(value.split())


def _required(
    fields: dict[str, Field], key: str, context: str, problems: list[str]
) -> Field | None:
    field = fields.get(key)
    if field is None:
        problems.append(f"{context}: missing {key!r}")
    return field


def _expect_keys(
    fields: dict[str, Field], expected: Iterable[str], context: str, problems: list[str]
) -> None:
    expected_set = set(expected)
    actual = set(fields)
    if actual != expected_set:
        missing = sorted(expected_set - actual)
        unexpected = sorted(actual - expected_set)
        problems.append(
            f"{context}: field set changed (missing={missing}, unexpected={unexpected})"
        )


def _expect_scalar(
    fields: dict[str, Field],
    key: str,
    expected: str,
    context: str,
    problems: list[str],
) -> None:
    field = _required(fields, key, context, problems)
    if field is not None and _scalar(field, f"{context}.{key}") != expected:
        problems.append(f"{context}.{key}: changed")


def _expect_scalar_map(
    field: Field,
    expected: dict[str, str],
    context: str,
    problems: list[str],
) -> None:
    fields = _mapping(field, context)
    _expect_keys(fields, expected, context, problems)
    for key, value in expected.items():
        if key in fields and _scalar(fields[key], f"{context}.{key}") != value:
            problems.append(f"{context}.{key}: changed")


def _named_step(
    steps: Sequence[Step], name: str, context: str, problems: list[str]
) -> Step | None:
    found: list[Step] = []
    for step in steps:
        field = step.fields.get("name")
        if field is not None and _scalar(field, context) == name:
            found.append(step)
    if len(found) != 1:
        problems.append(f"{context}: expected exactly one step named {name!r}")
        return None
    return found[0]


def _step_identity(step: Step, context: str) -> str:
    name = step.fields.get("name")
    if name is not None:
        return f"name:{_scalar(name, context)}"
    uses = step.fields.get("uses")
    if uses is not None:
        return f"uses:{_scalar(uses, context)}"
    return "<unnamed>"


def _expect_simple_step(
    step: Step,
    expected: dict[str, str],
    context: str,
    problems: list[str],
) -> None:
    _expect_keys(step.fields, expected, context, problems)
    for key, value in expected.items():
        field = step.fields.get(key)
        if field is not None and _scalar(field, f"{context}.{key}") != value:
            problems.append(f"{context}.{key}: changed")


def _validate_ci(root: dict[str, Field], problems: list[str]) -> None:
    _expect_keys(root, {"name", "on", "permissions", "jobs"}, "ci", problems)
    _expect_scalar(root, "name", "CI", "ci", problems)
    on = _required(root, "on", "ci", problems)
    permissions = _required(root, "permissions", "ci", problems)
    jobs = _required(root, "jobs", "ci", problems)
    if on is None or permissions is None or jobs is None:
        return

    triggers = _mapping(on, "ci.on")
    _expect_keys(
        triggers,
        {"pull_request", "push", "workflow_dispatch", "workflow_call"},
        "ci.on",
        problems,
    )
    for trigger in ("workflow_dispatch", "workflow_call"):
        field = triggers.get(trigger)
        if field is not None and _scalar(field, f"ci.on.{trigger}") != "":
            problems.append(f"ci.on.{trigger}: must not declare inputs or options")
    for trigger in ("pull_request", "push"):
        field = triggers.get(trigger)
        if field is not None:
            _expect_scalar_map(
                field, {"branches": "[main]"}, f"ci.on.{trigger}", problems
            )

    permission_fields = _mapping(permissions, "ci.permissions")
    _expect_keys(permission_fields, {"contents"}, "ci.permissions", problems)
    contents = permission_fields.get("contents")
    if contents is not None and _scalar(contents, "ci.permissions.contents") != "read":
        problems.append("ci.permissions.contents: must be read")

    job_fields = _mapping(jobs, "ci.jobs")
    _expect_keys(job_fields, {"test"}, "ci.jobs", problems)
    test = job_fields.get("test")
    if test is None:
        return
    test_fields = _mapping(test, "ci.jobs.test")
    _expect_keys(
        test_fields, {"runs-on", "defaults", "steps"}, "ci.jobs.test", problems
    )
    _expect_scalar(test_fields, "runs-on", "windows-latest", "ci.jobs.test", problems)
    defaults = _required(test_fields, "defaults", "ci.jobs.test", problems)
    if defaults is not None:
        default_fields = _mapping(defaults, "ci.jobs.test.defaults")
        _expect_keys(default_fields, {"run"}, "ci.jobs.test.defaults", problems)
        run_defaults = default_fields.get("run")
        if run_defaults is not None:
            _expect_scalar_map(
                run_defaults,
                {"working-directory": "apps/save-editor"},
                "ci.jobs.test.defaults.run",
                problems,
            )
    steps_field = _required(test_fields, "steps", "ci.jobs.test", problems)
    if steps_field is None:
        return
    steps = _parse_steps(steps_field, "ci.jobs.test.steps")
    expected_order = (
        "uses:actions/checkout@v4",
        "name:Install Rust",
        "name:Cache cargo",
        "name:Install Flutter",
        "name:Run all tests (rust, tools, analyze, flutter)",
        "name:gore-mod-studio analyze + test",
        "name:gore-mod-manager analyze + test",
        "name:Docs, plugin, and release contracts",
    )
    actual_order = tuple(
        _step_identity(step, "ci.jobs.test.steps") for step in steps
    )
    if actual_order != expected_order:
        problems.append(
            f"ci.jobs.test.steps: exact step order changed ({actual_order!r})"
        )
    if len(steps) != len(expected_order):
        return

    _expect_simple_step(
        steps[0], {"uses": "actions/checkout@v4"}, "ci checkout", problems
    )
    _expect_simple_step(
        steps[1],
        {"name": "Install Rust", "uses": "dtolnay/rust-toolchain@stable"},
        "ci rust setup",
        problems,
    )
    _expect_simple_step(
        steps[2],
        {"name": "Cache cargo", "uses": "Swatinem/rust-cache@v2"},
        "ci cargo cache",
        problems,
    )
    _expect_keys(
        steps[3].fields, {"name", "uses", "with"}, "ci flutter setup", problems
    )
    _expect_scalar(steps[3].fields, "name", "Install Flutter", "ci flutter setup", problems)
    _expect_scalar(
        steps[3].fields,
        "uses",
        "subosito/flutter-action@v2",
        "ci flutter setup",
        problems,
    )
    flutter_with = steps[3].fields.get("with")
    if flutter_with is not None:
        _expect_scalar_map(
            flutter_with,
            {"flutter-version": "3.44.0", "channel": "stable", "cache": "true"},
            "ci flutter setup.with",
            problems,
        )
    _expect_simple_step(
        steps[4],
        {
            "name": "Run all tests (rust, tools, analyze, flutter)",
            "run": "python test.py all",
        },
        "ci primary tests",
        problems,
    )
    flutter_commands = "flutter pub get\nflutter analyze\nflutter test"
    _expect_simple_step(
        steps[5],
        {
            "name": "gore-mod-studio analyze + test",
            "working-directory": "apps/mod-studio",
            "run": flutter_commands,
        },
        "ci mod studio",
        problems,
    )
    _expect_simple_step(
        steps[6],
        {
            "name": "gore-mod-manager analyze + test",
            "working-directory": "apps/mod-manager",
            "run": flutter_commands,
        },
        "ci mod manager",
        problems,
    )
    expected_run = (
        "python scripts/check_docs_links.py\n"
        "python scripts/check_plugin.py\n"
        "python scripts/check_release_workflow.py\n"
        'python -m unittest discover -s scripts -p "test_*.py" -v'
    )
    _expect_simple_step(
        steps[7],
        {
            "name": "Docs, plugin, and release contracts",
            "working-directory": ".",
            "run": expected_run,
        },
        "ci repository contracts",
        problems,
    )
    final_run = steps[7].fields.get("run")
    if final_run is None or _scalar(final_run, "ci repository contracts") != expected_run:
        problems.append("ci: final repository-check command list changed")


def _validate_release_input(root: dict[str, Field], problems: list[str]) -> None:
    on = _required(root, "on", "release", problems)
    if on is None:
        return
    triggers = _mapping(on, "release.on")
    _expect_keys(triggers, {"push", "workflow_dispatch"}, "release.on", problems)
    push = _required(triggers, "push", "release.on", problems)
    if push is not None:
        push_fields = _mapping(push, "release.on.push")
        _expect_keys(push_fields, {"tags"}, "release.on.push", problems)
        tags = push_fields.get("tags")
        if tags is not None:
            expected_tags = [f'"{contract.tag_prefix}*"' for contract in PRODUCTS.values()]
            actual_tags = _sequence(tags, "release.on.push.tags")
            if len(actual_tags) != len(expected_tags) or set(actual_tags) != set(
                expected_tags
            ):
                problems.append("release.on.push.tags: exact product tag set changed")
    dispatch = _required(triggers, "workflow_dispatch", "release.on", problems)
    if dispatch is None:
        return
    dispatch_fields = _mapping(dispatch, "release.on.workflow_dispatch")
    inputs = _required(
        dispatch_fields, "inputs", "release.on.workflow_dispatch", problems
    )
    if inputs is None:
        return
    input_fields = _mapping(inputs, "release.on.workflow_dispatch.inputs")
    _expect_keys(
        input_fields,
        {"project"},
        "release.on.workflow_dispatch.inputs",
        problems,
    )
    project = input_fields.get("project")
    if project is None:
        return
    project_fields = _mapping(project, "release input project")
    _expect_keys(
        project_fields,
        {"description", "type", "required", "options"},
        "release input project",
        problems,
    )
    description = _required(
        project_fields, "description", "release input project", problems
    )
    required = _required(project_fields, "required", "release input project", problems)
    kind = _required(project_fields, "type", "release input project", problems)
    options = _required(project_fields, "options", "release input project", problems)
    if description is not None and _scalar(description, "release input description") != (
        '"Smoke-build a project without publishing"'
    ):
        problems.append("release input project: description changed")
    if required is not None and _scalar(required, "release input required") != "true":
        problems.append("release input project: required must be true")
    if kind is not None and _scalar(kind, "release input type") != "choice":
        problems.append("release input project: type must remain choice")
    expected_options = "[" + ", ".join(PRODUCTS) + "]"
    if options is not None and _scalar(options, "release input options") != expected_options:
        problems.append("release input project: product options changed")


def _expected_job_if(product: str, contract: ProductContract) -> str:
    return (
        "(github.event_name == 'push' && github.ref_type == 'tag' && "
        f"startsWith(github.ref_name, '{contract.tag_prefix}')) || "
        "(github.event_name == 'workflow_dispatch' && "
        f"inputs.project == '{product}')"
    )


def _validate_release_step_allowlist(
    product: str,
    contract: ProductContract,
    steps: Sequence[Step],
    problems: list[str],
) -> None:
    context = f"release.jobs.{product}.steps"
    if contract.appcast_command is not None:
        expected_order = (
            "uses:actions/checkout@v4",
            "name:Resolve and verify version",
            "uses:dtolnay/rust-toolchain@stable",
            "uses:Swatinem/rust-cache@v2",
            "uses:subosito/flutter-action@v2",
            "name:Install Inno Setup",
            f"name:{contract.build_step}",
            "name:Extract release notes from CHANGELOG.md",
            "name:Generate signed appcast",
            "name:Upload build artifact",
            "name:Publish GitHub release",
            f"name:Update {product} appcast feed",
        )
    else:
        expected_order = (
            "uses:actions/checkout@v4",
            "name:Resolve and verify version",
            "uses:dtolnay/rust-toolchain@stable",
            "uses:Swatinem/rust-cache@v2",
            f"name:{contract.build_step}",
            "name:Extract release notes from CHANGELOG.md",
            "name:Upload build artifact",
            "name:Publish GitHub release",
        )
    actual_order = tuple(_step_identity(step, context) for step in steps)
    if actual_order != expected_order:
        problems.append(f"{context}: exact step order changed ({actual_order!r})")
    if len(steps) != len(expected_order):
        return

    _expect_simple_step(
        steps[0], {"uses": "actions/checkout@v4"}, f"{context}.checkout", problems
    )
    _expect_simple_step(
        steps[1],
        {
            "name": "Resolve and verify version",
            "id": "version",
            "shell": "pwsh",
            "run": contract.version_command,
        },
        f"{context}.version",
        problems,
    )
    _expect_simple_step(
        steps[2],
        {"uses": "dtolnay/rust-toolchain@stable"},
        f"{context}.rust",
        problems,
    )
    _expect_simple_step(
        steps[3],
        {"uses": "Swatinem/rust-cache@v2"},
        f"{context}.cargo-cache",
        problems,
    )

    if contract.appcast_command is not None:
        _expect_keys(
            steps[4].fields,
            {"uses", "with"},
            f"{context}.flutter",
            problems,
        )
        _expect_scalar(
            steps[4].fields,
            "uses",
            "subosito/flutter-action@v2",
            f"{context}.flutter",
            problems,
        )
        flutter_with = steps[4].fields.get("with")
        if flutter_with is not None:
            _expect_scalar_map(
                flutter_with,
                {
                    "flutter-version": "3.44.0",
                    "channel": "stable",
                    "cache": "true",
                },
                f"{context}.flutter.with",
                problems,
            )
        _expect_simple_step(
            steps[5],
            {
                "name": "Install Inno Setup",
                "run": "choco install innosetup --no-progress -y",
            },
            f"{context}.inno",
            problems,
        )
        notes_index = 7
    else:
        notes_index = 5

    _expect_simple_step(
        steps[notes_index],
        {
            "name": "Extract release notes from CHANGELOG.md",
            "shell": "pwsh",
            "run": contract.release_notes_command,
        },
        f"{context}.release-notes",
        problems,
    )


def _validate_product_steps(
    product: str,
    contract: ProductContract,
    steps: Sequence[Step],
    problems: list[str],
) -> None:
    context = f"release.jobs.{product}"
    _validate_release_step_allowlist(product, contract, steps, problems)

    build = _named_step(steps, contract.build_step, context, problems)
    if build is not None:
        _expect_keys(build.fields, {"name", "run"}, f"{context} build", problems)
        run = build.fields.get("run")
        if run is None or _scalar(run, f"{context} build") != contract.build_command:
            problems.append(f"{context}: build command changed")
    build_like = [
        step
        for step in steps
        if (run := step.fields.get("run")) is not None
        and "build.py" in _scalar(run, context)
    ]
    if len(build_like) != 1 or (build is not None and build_like[0] is not build):
        problems.append(f"{context}: unexpected or duplicate build command")

    upload = _named_step(steps, "Upload build artifact", context, problems)
    if upload is not None:
        _expect_keys(
            upload.fields, {"name", "uses", "with"}, f"{context} upload", problems
        )
        uses = upload.fields.get("uses")
        with_field = upload.fields.get("with")
        if uses is None or _scalar(uses, f"{context} upload") != "actions/upload-artifact@v4":
            problems.append(f"{context}: upload action changed")
        if with_field is not None:
            with_fields = _mapping(with_field, f"{context} upload.with")
            _expect_keys(
                with_fields, {"name", "path"}, f"{context} upload.with", problems
            )
            name = with_fields.get("name")
            path = with_fields.get("path")
            if name is None or _scalar(name, context) != contract.upload_name:
                problems.append(f"{context}: upload artifact name changed")
            if path is None or _scalar(path, context) != contract.upload_paths:
                problems.append(f"{context}: upload artifact paths changed")
    upload_like = [
        step
        for step in steps
        if (uses := step.fields.get("uses")) is not None
        and "actions/upload-artifact@" in _scalar(uses, context)
    ]
    if len(upload_like) != 1 or (upload is not None and upload_like[0] is not upload):
        problems.append(f"{context}: unexpected or duplicate artifact upload")

    publish = _named_step(steps, "Publish GitHub release", context, problems)
    if publish is not None:
        _expect_keys(
            publish.fields,
            {"name", "if", "uses", "with"},
            f"{context} publish",
            problems,
        )
        guard = publish.fields.get("if")
        uses = publish.fields.get("uses")
        with_field = publish.fields.get("with")
        if guard is None or _normalise_space(_scalar(guard, context)) != PUBLISH_GUARD:
            problems.append(f"{context}: publish step lost its push-tag-only guard")
        if uses is None or _scalar(uses, context) != "softprops/action-gh-release@v2":
            problems.append(f"{context}: release action changed")
        if with_field is not None:
            with_fields = _mapping(with_field, f"{context} publish.with")
            _expect_keys(
                with_fields,
                {"make_latest", "files", "body_path"},
                f"{context} publish.with",
                problems,
            )
            latest = with_fields.get("make_latest")
            files = with_fields.get("files")
            body = with_fields.get("body_path")
            if latest is None or _scalar(latest, context) != contract.make_latest:
                problems.append(f"{context}: make_latest changed")
            if files is None or _scalar(files, context) != contract.publish_files:
                problems.append(f"{context}: published release files changed")
            if body is None or _scalar(body, context) != contract.publish_body:
                problems.append(f"{context}: release body path changed")
    release_like = [
        step
        for step in steps
        if (uses := step.fields.get("uses")) is not None
        and "action-gh-release" in _scalar(uses, context)
    ]
    if len(release_like) != 1 or (publish is not None and release_like[0] is not publish):
        problems.append(f"{context}: unexpected or duplicate GitHub release action")

    appcast_like = [
        step
        for step in steps
        if (run := step.fields.get("run")) is not None
        and "scripts/appcast.py" in _scalar(run, context)
    ]
    if contract.appcast_command is None:
        if appcast_like:
            problems.append(f"{context}: unexpected appcast generation")
    else:
        appcast = _named_step(steps, "Generate signed appcast", context, problems)
        if appcast is not None:
            _expect_keys(
                appcast.fields,
                {"name", "env", "run"},
                f"{context} appcast",
                problems,
            )
            env = appcast.fields.get("env")
            run = appcast.fields.get("run")
            if env is not None:
                env_fields = _mapping(env, f"{context} appcast.env")
                _expect_keys(
                    env_fields,
                    {"WINSPARKLE_DSA_PRIV_KEY_B64"},
                    f"{context} appcast.env",
                    problems,
                )
                secret = env_fields.get("WINSPARKLE_DSA_PRIV_KEY_B64")
                if secret is not None and _scalar(secret, context) != (
                    "${{ secrets.WINSPARKLE_DSA_PRIV_KEY_B64 }}"
                ):
                    problems.append(f"{context}: appcast signing secret binding changed")
            if run is None or _normalise_space(_scalar(run, context)) != contract.appcast_command:
                problems.append(f"{context}: signed appcast command changed")
        if len(appcast_like) != 1 or (
            appcast is not None and appcast_like[0] is not appcast
        ):
            problems.append(f"{context}: unexpected or duplicate appcast generation")

    feed_like = [
        step
        for step in steps
        if (run := step.fields.get("run")) is not None
        and "gh release " in _scalar(run, context)
    ]
    if contract.feed_command is None:
        if feed_like:
            problems.append(f"{context}: unexpected appcast feed publication")
    else:
        feed_name = f"Update {product} appcast feed"
        feed = _named_step(steps, feed_name, context, problems)
        if feed is not None:
            _expect_keys(
                feed.fields,
                {"name", "if", "shell", "env", "run"},
                f"{context} feed",
                problems,
            )
            guard = feed.fields.get("if")
            shell = feed.fields.get("shell")
            env = feed.fields.get("env")
            run = feed.fields.get("run")
            if guard is None or _normalise_space(_scalar(guard, context)) != PUBLISH_GUARD:
                problems.append(f"{context}: appcast feed lost its push-tag-only guard")
            if shell is None or _scalar(shell, context) != "pwsh":
                problems.append(f"{context}: appcast feed shell changed")
            if env is not None:
                env_fields = _mapping(env, f"{context} feed.env")
                _expect_keys(env_fields, {"GH_TOKEN"}, f"{context} feed.env", problems)
                token = env_fields.get("GH_TOKEN")
                if token is None or _scalar(token, context) != "${{ github.token }}":
                    problems.append(f"{context}: appcast feed token source changed")
            if run is None or _scalar(run, context) != contract.feed_command:
                problems.append(f"{context}: appcast feed command changed")
        if len(feed_like) != 1 or (feed is not None and feed_like[0] is not feed):
            problems.append(f"{context}: unexpected or duplicate appcast feed command")


def _validate_release(root: dict[str, Field], problems: list[str]) -> None:
    _expect_keys(
        root, {"name", "on", "permissions", "env", "jobs"}, "release", problems
    )
    _expect_scalar(root, "name", "Release", "release", problems)
    _validate_release_input(root, problems)
    permissions = _required(root, "permissions", "release", problems)
    if permissions is not None:
        _expect_scalar_map(
            permissions, {"contents": "write"}, "release.permissions", problems
        )
    env = _required(root, "env", "release", problems)
    if env is not None:
        _expect_scalar_map(
            env,
            {
                "GORE_SIGN": '"1"',
                "TRUSTED_SIGNING_ENDPOINT": "${{ secrets.TRUSTED_SIGNING_ENDPOINT }}",
                "TRUSTED_SIGNING_ACCOUNT": "${{ secrets.TRUSTED_SIGNING_ACCOUNT }}",
                "TRUSTED_SIGNING_PROFILE": "${{ secrets.TRUSTED_SIGNING_PROFILE }}",
                "AZURE_TENANT_ID": "${{ secrets.AZURE_TENANT_ID }}",
                "AZURE_CLIENT_ID": "${{ secrets.AZURE_CLIENT_ID }}",
                "AZURE_CLIENT_SECRET": "${{ secrets.AZURE_CLIENT_SECRET }}",
            },
            "release.env",
            problems,
        )
    jobs = _required(root, "jobs", "release", problems)
    if jobs is None:
        return
    job_fields = _mapping(jobs, "release.jobs")
    expected_jobs = {QUALITY_JOB, *PRODUCTS}
    _expect_keys(job_fields, expected_jobs, "release.jobs", problems)

    quality = job_fields.get(QUALITY_JOB)
    if quality is not None:
        quality_fields = _mapping(quality, f"release.jobs.{QUALITY_JOB}")
        _expect_keys(
            quality_fields,
            {"name", "permissions", "uses"},
            f"release.jobs.{QUALITY_JOB}",
            problems,
        )
        name = quality_fields.get("name")
        uses = quality_fields.get("uses")
        permissions = quality_fields.get("permissions")
        if name is None or _scalar(name, QUALITY_JOB) != "CI quality gates":
            problems.append(f"release.jobs.{QUALITY_JOB}: display name changed")
        if uses is None or _scalar(uses, QUALITY_JOB) != REUSABLE_CI:
            problems.append(
                f"release.jobs.{QUALITY_JOB}: must call the exact local CI workflow"
            )
        if permissions is not None:
            permission_fields = _mapping(permissions, f"{QUALITY_JOB}.permissions")
            _expect_keys(
                permission_fields,
                {"contents"},
                f"{QUALITY_JOB}.permissions",
                problems,
            )
            contents = permission_fields.get("contents")
            if contents is None or _scalar(contents, QUALITY_JOB) != "read":
                problems.append(f"release.jobs.{QUALITY_JOB}: contents must be read")

    for product, contract in PRODUCTS.items():
        job = job_fields.get(product)
        if job is None:
            continue
        context = f"release.jobs.{product}"
        fields = _mapping(job, context)
        _expect_keys(
            fields, {"needs", "if", "runs-on", "steps"}, context, problems
        )
        needs = _required(fields, "needs", context, problems)
        condition = _required(fields, "if", context, problems)
        runs_on = _required(fields, "runs-on", context, problems)
        steps_field = _required(fields, "steps", context, problems)
        if needs is not None and _scalar(needs, context) != QUALITY_JOB:
            problems.append(f"{context}: needs must be the scalar {QUALITY_JOB!r}")
        if condition is not None:
            actual_if = _normalise_space(_scalar(condition, context))
            expected_if = _expected_job_if(product, contract)
            if actual_if != expected_if:
                problems.append(
                    f"{context}: tag/dispatch product selector changed or weakened"
                )
        if runs_on is not None and _scalar(runs_on, context) != "windows-latest":
            problems.append(f"{context}: runner changed")
        if steps_field is not None:
            steps = _parse_steps(steps_field, f"{context}.steps")
            _validate_product_steps(product, contract, steps, problems)


def validate_workflows(ci_text: str, release_text: str) -> list[str]:
    """Return contract violations; an empty list means the workflows are safe."""
    try:
        ci_root = _parse_fields(_source(ci_text, "ci.yml"), 0, "ci.yml")
        release_root = _parse_fields(
            _source(release_text, "release.yml"), 0, "release.yml"
        )
        problems: list[str] = []
        _validate_ci(ci_root, problems)
        _validate_release(release_root, problems)
        return problems
    except WorkflowParseError as error:
        return [str(error)]


def main() -> int:
    try:
        ci_text = CI_PATH.read_text(encoding="utf-8")
        release_text = RELEASE_PATH.read_text(encoding="utf-8")
    except OSError as error:
        print(f"release workflow check could not read its inputs: {error}", file=sys.stderr)
        return 1

    problems = validate_workflows(ci_text, release_text)
    if problems:
        for problem in problems:
            print(problem, file=sys.stderr)
        print(f"{len(problems)} release workflow contract violation(s).", file=sys.stderr)
        return 1

    print("OK: release jobs are gated by the exact normal CI workflow.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
