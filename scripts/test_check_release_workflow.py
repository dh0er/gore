from __future__ import annotations

from pathlib import Path
import posixpath
import re
import unittest

from check_release_workflow import (
    APPCAST_KEYS,
    DOWNLOAD_TOOLS,
    PRODUCTS,
    PUBLISH_GUARD,
    UNRELEASED_PRODUCTS,
    validate_appcast_key_resources,
    validate_download_table,
    validate_workflows,
)


ROOT = Path(__file__).resolve().parent.parent

CLI_ROW_PATTERN = "(?m)^" + re.escape("| **CLI** |") + ".*" + chr(10)


def replace_once(text: str, old: str, new: str) -> str:
    count = text.count(old)
    if count != 1:
        raise AssertionError(f"expected one occurrence, found {count}: {old!r}")
    return text.replace(old, new, 1)


def replace_nth(text: str, old: str, new: str, index: int) -> str:
    starts = [match.start() for match in re.finditer(re.escape(old), text)]
    if index >= len(starts):
        raise AssertionError(f"occurrence {index} missing for {old!r}")
    start = starts[index]
    return text[:start] + new + text[start + len(old) :]


def mutate_job(text: str, job: str, old: str, new: str) -> str:
    pattern = re.compile(
        rf"(?ms)(^  {re.escape(job)}:\n)(.*?)(?=^  [A-Za-z_][A-Za-z0-9_-]*:\n|\Z)"
    )
    match = pattern.search(text)
    if match is None:
        raise AssertionError(f"job {job!r} not found")
    body = replace_once(match.group(2), old, new)
    return text[: match.start(2)] + body + text[match.end(2) :]


def remove_job(text: str, job: str) -> str:
    pattern = re.compile(
        rf"(?ms)^  {re.escape(job)}:\n.*?(?=^  [A-Za-z_][A-Za-z0-9_-]*:\n|\Z)"
    )
    changed, count = pattern.subn("", text, count=1)
    if count != 1:
        raise AssertionError(f"job {job!r} not found")
    return changed


class DownloadTableContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.readme = (ROOT / "README.md").read_text(encoding="utf-8")

    def assert_invalid(self, readme: str, ref: str | None, mentions: str) -> None:
        problems = validate_download_table(readme, ref)
        self.assertTrue(problems, "mutated README unexpectedly passed")
        self.assertTrue(
            any(mentions in problem for problem in problems),
            f"no problem mentioned {mentions!r}: {problems}",
        )

    def test_current_readme_passes(self) -> None:
        self.assertEqual(validate_download_table(self.readme), [])

    def test_every_release_product_is_covered(self) -> None:
        self.assertEqual(
            set(PRODUCTS), set(DOWNLOAD_TOOLS.values()) | UNRELEASED_PRODUCTS
        )

    def test_release_tag_must_match_the_advertised_version(self) -> None:
        self.assertEqual(
            validate_download_table(self.readme, "gore-save-editor-v1.3.0"), []
        )
        self.assert_invalid(
            self.readme, "gore-save-editor-v1.4.0", "the release tag is"
        )

    def test_releasing_an_unreleased_product_requires_a_row(self) -> None:
        self.assert_invalid(
            self.readme,
            "gore-mod-studio-v0.1.0",
            "must move out of the unreleased list",
        )

    def test_link_must_point_at_its_own_release_tag(self) -> None:
        self.assert_invalid(
            replace_once(
                self.readme,
                "releases/tag/gore-cli-v0.2.3)",
                # Deliberately wrong test-only version: the checker must reject this link.
                "releases/tag/gore-cli-v9.9.9)",
            ),
            None,
            "CLI must link to",
        )

    def test_link_text_must_be_the_release_tag(self) -> None:
        self.assert_invalid(
            replace_once(self.readme, "[gore-cli-v0.2.3]", "[latest]"),
            None,
            "link text must be the release tag",
        )

    def test_every_tool_needs_a_row(self) -> None:
        readme = re.sub(CLI_ROW_PATTERN, "", self.readme, count=1)
        self.assert_invalid(readme, None, "no CLI row")

    def test_download_section_and_header_are_pinned(self) -> None:
        self.assert_invalid(
            replace_once(self.readme, "## ⬇️ Downloads", "## Downloads"),
            None,
            "missing '## ⬇️ Downloads' section",
        )
        self.assert_invalid(
            replace_once(
                self.readme,
                "| Tool | Version | Release page |",
                "| Tool | Release page | Version |",
            ),
            None,
            "download table must start with",
        )

    def test_unreadable_and_unknown_rows_fail_closed(self) -> None:
        self.assert_invalid(
            replace_once(self.readme, "| **CLI** | 0.2.3 |", "| **CLI** | v0.2.3 |"),
            None,
            "unreadable download row",
        )
        self.assert_invalid(
            replace_once(self.readme, "| **CLI** |", "| **gore.exe** |"),
            None,
            "unknown download tool",
        )


class ReleaseWorkflowContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.ci = (ROOT / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")
        cls.release = (ROOT / ".github" / "workflows" / "release.yml").read_text(
            encoding="utf-8"
        )
        cls.runner_resources = {
            product: (ROOT / contract.runner_rc).read_text(encoding="utf-8")
            for product, contract in APPCAST_KEYS.items()
        }

    def assert_invalid(
        self,
        *,
        ci: str | None = None,
        release: str | None = None,
        mentions: str | None = None,
    ) -> list[str]:
        problems = validate_workflows(ci or self.ci, release or self.release)
        self.assertTrue(problems, "mutated workflows unexpectedly passed")
        if mentions is not None:
            self.assertTrue(
                any(mentions in problem for problem in problems),
                f"no problem mentioned {mentions!r}: {problems}",
            )
        return problems

    def test_current_workflows_pass(self) -> None:
        self.assertEqual(validate_workflows(self.ci, self.release), [])
        self.assertEqual(validate_appcast_key_resources(self.runner_resources), [])

    def test_ci_must_be_reusable_and_read_only(self) -> None:
        without_call = replace_once(self.ci, "  workflow_call:\n", "")
        self.assert_invalid(ci=without_call, mentions="workflow_call")

        writable = replace_once(
            self.ci, "permissions:\n  contents: read", "permissions:\n  contents: write"
        )
        self.assert_invalid(ci=writable, mentions="must be read")

    def test_ci_must_keep_the_contract_checks_as_its_last_step(self) -> None:
        changed = replace_once(
            self.ci,
            '          python -m unittest discover -s scripts -p "test_*.py" -v\n',
            "",
        )
        self.assert_invalid(ci=changed, mentions="command list changed")

    def test_ci_primary_test_command_and_step_allowlist_are_pinned(self) -> None:
        removed = replace_once(
            self.ci,
            "        run: python test.py all",
            "        run: echo tests-skipped",
        )
        self.assert_invalid(ci=removed, mentions="ci primary tests.run: changed")

        continue_on_error = replace_once(
            self.ci,
            "        run: python test.py all\n",
            "        run: python test.py all\n        continue-on-error: true\n",
        )
        self.assert_invalid(ci=continue_on_error, mentions="field set changed")

        unknown = replace_once(
            self.ci,
            "      - uses: actions/checkout@v4\n",
            "      - uses: actions/checkout@v4\n\n      - run: echo unknown-step\n",
        )
        self.assert_invalid(ci=unknown, mentions="exact step order changed")

    def test_ci_and_release_checkout_cannot_select_another_ref(self) -> None:
        ci_ref = replace_once(
            self.ci,
            "      - uses: actions/checkout@v4\n",
            "      - uses: actions/checkout@v4\n        with:\n          ref: main\n",
        )
        self.assert_invalid(ci=ci_ref, mentions="field set changed")

        for product in PRODUCTS:
            with self.subTest(product=product):
                release_ref = mutate_job(
                    self.release,
                    product,
                    "      - uses: actions/checkout@v4\n",
                    "      - uses: actions/checkout@v4\n        with:\n          ref: main\n",
                )
                self.assert_invalid(release=release_ref, mentions="field set changed")

    def test_quality_job_must_call_exact_local_ci_without_bypass_fields(self) -> None:
        wrong_path = mutate_job(
            self.release,
            "quality-gates",
            "uses: ./.github/workflows/ci.yml",
            "uses: ./.github/workflows/other.yml",
        )
        self.assert_invalid(release=wrong_path, mentions="exact local CI")

        writable = mutate_job(
            self.release, "quality-gates", "contents: read", "contents: write"
        )
        self.assert_invalid(release=writable, mentions="contents must be read")

        conditional = mutate_job(
            self.release,
            "quality-gates",
            "name: CI quality gates",
            "name: CI quality gates\n    if: github.event_name == 'workflow_dispatch'",
        )
        self.assert_invalid(release=conditional, mentions="field set changed")

        secrets = mutate_job(
            self.release,
            "quality-gates",
            "uses: ./.github/workflows/ci.yml",
            "secrets: inherit\n    uses: ./.github/workflows/ci.yml",
        )
        self.assert_invalid(release=secrets, mentions="field set changed")

    def test_exact_release_job_set_is_required(self) -> None:
        for product in PRODUCTS:
            with self.subTest(missing=product):
                self.assert_invalid(
                    release=remove_job(self.release, product), mentions="missing="
                )

        unknown = replace_once(
            self.release,
            "jobs:\n",
            "jobs:\n  surprise-release:\n    runs-on: windows-latest\n",
        )
        self.assert_invalid(release=unknown, mentions="unexpected=")

    def test_every_product_needs_exact_quality_job_scalar(self) -> None:
        for product in PRODUCTS:
            with self.subTest(product=product, mutation="missing"):
                missing = mutate_job(
                    self.release, product, "    needs: quality-gates\n", ""
                )
                self.assert_invalid(release=missing, mentions="missing 'needs'")
            with self.subTest(product=product, mutation="wrong"):
                wrong = mutate_job(
                    self.release,
                    product,
                    "needs: quality-gates",
                    "needs: some-other-job",
                )
                self.assert_invalid(release=wrong, mentions="needs must be")
            with self.subTest(product=product, mutation="list"):
                sequence = mutate_job(
                    self.release,
                    product,
                    "needs: quality-gates",
                    "needs: [quality-gates]",
                )
                self.assert_invalid(release=sequence, mentions="needs must be")

    def test_product_selectors_require_push_tag_prefix_or_exact_dispatch(self) -> None:
        mutations = (
            (
                "if: >-\n      (github.event_name == 'push' &&",
                "if: >-\n      (github.event_name != 'pull_request' &&",
            ),
            ("github.ref_type == 'tag'", "github.ref_type != 'branch'"),
            ("github.event_name == 'workflow_dispatch'", "github.event_name != 'push'"),
        )
        for product in PRODUCTS:
            for old, new in mutations:
                with self.subTest(product=product, mutation=old):
                    changed = mutate_job(self.release, product, old, new)
                    self.assert_invalid(release=changed, mentions="selector changed")
            with self.subTest(product=product, mutation="prefix"):
                prefix = PRODUCTS[product].tag_prefix
                changed = mutate_job(
                    self.release,
                    product,
                    f"startsWith(github.ref_name, '{prefix}')",
                    f"startsWith(github.ref_name, 'wrong-{prefix}')",
                )
                self.assert_invalid(release=changed, mentions="selector changed")
            with self.subTest(product=product, mutation="selection"):
                changed = mutate_job(
                    self.release,
                    product,
                    f"inputs.project == '{product}'",
                    "inputs.project != ''",
                )
                self.assert_invalid(release=changed, mentions="selector changed")

    def test_folded_selector_is_compared_by_whitespace_semantics(self) -> None:
        reformatted = mutate_job(
            self.release,
            "gore-save-editor",
            "github.ref_type == 'tag' &&",
            "github.ref_type    ==    'tag'    &&",
        )
        self.assertEqual(validate_workflows(self.ci, reformatted), [])

    def test_dispatch_project_is_required_and_closed(self) -> None:
        optional = replace_once(
            self.release, "        required: true", "        required: false"
        )
        self.assert_invalid(release=optional, mentions="required must be true")

        extra = replace_once(
            self.release,
            "options: [gore-save-editor, gore-mod-studio, gore-mod-manager, gore-cli]",
            "options: [gore-save-editor, gore-mod-studio, gore-mod-manager, gore-cli, surprise]",
        )
        self.assert_invalid(release=extra, mentions="options changed")

    def test_release_tag_triggers_permissions_and_signing_env_are_pinned(self) -> None:
        removed_tag = replace_once(self.release, '      - "gore-cli-v*"\n', "")
        self.assert_invalid(release=removed_tag, mentions="tag set changed")

        changed_tag = replace_once(
            self.release, '      - "gore-mod-studio-v*"', '      - "gore-mod-studio-*"'
        )
        self.assert_invalid(release=changed_tag, mentions="tag set changed")

        unsigned = replace_once(self.release, '  GORE_SIGN: "1"', '  GORE_SIGN: "0"')
        self.assert_invalid(release=unsigned, mentions="release.env.GORE_SIGN: changed")

        readonly = replace_once(
            self.release,
            "permissions:\n  contents: write",
            "permissions:\n  contents: read",
        )
        self.assert_invalid(
            release=readonly, mentions="release.permissions.contents: changed"
        )

    def test_cli_tag_builds_compiler_and_distribution_through_build_py(self) -> None:
        renamed = mutate_job(
            self.release,
            "gore-cli",
            "name: Build compiler and distribution",
            "name: Build distribution",
        )
        self.assert_invalid(release=renamed, mentions="exact step order changed")

        bypassed = mutate_job(
            self.release,
            "gore-cli",
            "run: python build.py gore-cli dist",
            "run: cargo build --release",
        )
        self.assert_invalid(release=bypassed, mentions="build command changed")

    def test_all_publish_and_feed_steps_keep_push_tag_only_guard(self) -> None:
        self.assertEqual(self.release.count(PUBLISH_GUARD), 7)
        weakened = "startsWith(github.ref, 'refs/tags/')"
        for index in range(7):
            with self.subTest(step=index):
                changed = replace_nth(self.release, PUBLISH_GUARD, weakened, index)
                self.assert_invalid(release=changed, mentions="push-tag-only guard")

    def test_workflow_dispatch_from_a_tag_cannot_publish(self) -> None:
        # A dispatch can use a tag ref.  A ref-only guard would publish in that
        # case, so the event-name half of the guard is a security boundary.
        changed = replace_nth(
            self.release,
            PUBLISH_GUARD,
            "startsWith(github.ref, 'refs/tags/')",
            0,
        )
        self.assert_invalid(release=changed, mentions="push-tag-only guard")

    def test_release_rejects_unknown_publish_actions_and_steps(self) -> None:
        unknown_publish = mutate_job(
            self.release,
            "gore-cli",
            "uses: softprops/action-gh-release@v2",
            "uses: ncipollo/release-action@v1",
        )
        self.assert_invalid(release=unknown_publish, mentions="release action changed")

        unknown_step = mutate_job(
            self.release,
            "gore-cli",
            "      - uses: actions/checkout@v4\n",
            "      - uses: actions/checkout@v4\n\n      - run: echo unknown-step\n",
        )
        self.assert_invalid(release=unknown_step, mentions="exact step order changed")

        continue_on_error = mutate_job(
            self.release,
            "gore-cli",
            "        run: python build.py gore-cli dist\n",
            "        run: python build.py gore-cli dist\n        continue-on-error: true\n",
        )
        self.assert_invalid(release=continue_on_error, mentions="field set changed")

    def test_all_build_commands_are_pinned(self) -> None:
        for product, contract in PRODUCTS.items():
            with self.subTest(product=product):
                changed = mutate_job(
                    self.release,
                    product,
                    contract.build_command,
                    contract.build_command + " --changed",
                )
                self.assert_invalid(release=changed, mentions="build command changed")

    def test_manager_artifact_verification_is_pinned_before_upload(self) -> None:
        command = (
            "python scripts/verify_mod_manager_release.py "
            "--version ${{ steps.version.outputs.version }}"
        )
        removed = mutate_job(
            self.release,
            "gore-mod-manager",
            f"      - name: Verify release artifacts\n        run: {command}\n\n",
            "",
        )
        self.assert_invalid(release=removed, mentions="exact step order changed")

        bypassed = mutate_job(
            self.release,
            "gore-mod-manager",
            f"        run: {command}\n",
            f"        run: {command}\n        continue-on-error: true\n",
        )
        self.assert_invalid(release=bypassed, mentions="field set changed")

        wrong_version = mutate_job(
            self.release,
            "gore-mod-manager",
            command,
            "python scripts/verify_mod_manager_release.py --version 0.0.0",
        )
        self.assert_invalid(
            release=wrong_version, mentions="artifact verification.run: changed"
        )

    def test_all_upload_names_and_paths_are_pinned(self) -> None:
        for product, contract in PRODUCTS.items():
            with self.subTest(product=product, field="name"):
                changed = mutate_job(
                    self.release,
                    product,
                    f"name: {contract.upload_name}",
                    f"name: changed-{contract.upload_name}",
                )
                self.assert_invalid(release=changed, mentions="artifact name changed")
            with self.subTest(product=product, field="path"):
                first_path = contract.upload_paths.splitlines()[0]
                old = (
                    "          path: |\n"
                    if "\n" in contract.upload_paths
                    else f"          path: {first_path}\n"
                )
                changed = mutate_job(
                    self.release,
                    product,
                    old,
                    "          path: dist/changed/*.zip\n",
                )
                self.assert_invalid(release=changed)

    def test_release_files_and_body_are_pinned(self) -> None:
        for product, contract in PRODUCTS.items():
            with self.subTest(product=product, field="body"):
                changed = mutate_job(
                    self.release,
                    product,
                    f"body_path: {contract.publish_body}",
                    "body_path: dist/changed/RELEASE_NOTES.md",
                )
                self.assert_invalid(release=changed, mentions="body path changed")
            with self.subTest(product=product, field="files"):
                first_file = contract.publish_files.splitlines()[0]
                old = (
                    "          files: |\n"
                    if "\n" in contract.publish_files
                    else f"          files: {first_file}\n"
                )
                changed = mutate_job(
                    self.release, product, old, "          files: dist/changed/*.zip\n"
                )
                self.assert_invalid(release=changed)

    def test_only_manager_versioned_releases_are_prereleases(self) -> None:
        missing = mutate_job(
            self.release,
            "gore-mod-manager",
            '          prerelease: "true"\n',
            "",
        )
        self.assert_invalid(release=missing, mentions="prerelease flag changed")

        false = mutate_job(
            self.release,
            "gore-mod-manager",
            'prerelease: "true"',
            'prerelease: "false"',
        )
        self.assert_invalid(release=false, mentions="prerelease flag changed")

        for product in ("gore-save-editor", "gore-mod-studio", "gore-cli"):
            with self.subTest(product=product):
                changed = mutate_job(
                    self.release,
                    product,
                    "          make_latest: ",
                    '          prerelease: "true"\n          make_latest: ',
                )
                self.assert_invalid(release=changed, mentions="field set changed")

    def test_all_appcast_generation_and_feed_commands_are_pinned(self) -> None:
        for product, contract in PRODUCTS.items():
            if contract.appcast_command is None or contract.feed_command is None:
                continue
            with self.subTest(product=product, command="generate"):
                changed = mutate_job(
                    self.release,
                    product,
                    f"--title {product}",
                    "--title changed-product",
                )
                self.assert_invalid(release=changed, mentions="appcast command changed")
            with self.subTest(product=product, command="feed"):
                changed = mutate_job(
                    self.release,
                    product,
                    f"gh release upload {product}-appcast",
                    "gh release upload changed-appcast",
                )
                self.assert_invalid(release=changed, mentions="feed command changed")

    def test_all_appcasts_are_bound_to_their_embedded_public_key(self) -> None:
        for product, contract in APPCAST_KEYS.items():
            with self.subTest(product=product):
                changed = mutate_job(
                    self.release,
                    product,
                    f"--public-key {contract.public_key}",
                    "--public-key apps/changed/dsa_pub.pem",
                )
                self.assert_invalid(release=changed, mentions="appcast command changed")

    def test_runner_resources_are_bound_to_the_workflow_public_keys(self) -> None:
        for product, contract in APPCAST_KEYS.items():
            with self.subTest(product=product):
                relative_key = posixpath.relpath(
                    contract.public_key, posixpath.dirname(contract.runner_rc)
                )
                changed = dict(self.runner_resources)
                changed[product] = replace_once(
                    changed[product],
                    f'DSAPEM                  "{relative_key}"',
                    'DSAPEM                  "../../changed.pem"',
                )
                problems = validate_appcast_key_resources(changed)
                self.assertTrue(
                    any(contract.runner_rc in problem for problem in problems),
                    f"changed {product} Runner.rc unexpectedly passed: {problems}",
                )

    def test_duplicate_and_unsupported_yaml_shapes_fail_closed(self) -> None:
        duplicate = mutate_job(
            self.release,
            "gore-cli",
            "needs: quality-gates",
            "needs: quality-gates\n    needs: quality-gates",
        )
        self.assert_invalid(release=duplicate, mentions="duplicate key")

        flow_jobs = replace_once(self.release, "jobs:\n", "jobs: {}\n")
        self.assert_invalid(release=flow_jobs, mentions="expected an indented mapping")

        tabbed = replace_nth(
            self.release, "    needs: quality-gates", "\tneeds: quality-gates", 0
        )
        self.assert_invalid(release=tabbed, mentions="tabs are not accepted")


if __name__ == "__main__":
    unittest.main()
