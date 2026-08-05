from __future__ import annotations

from pathlib import Path
import re
import unittest

from check_release_workflow import PRODUCTS, PUBLISH_GUARD, validate_workflows


ROOT = Path(__file__).resolve().parent.parent


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


class ReleaseWorkflowContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.ci = (ROOT / ".github" / "workflows" / "ci.yml").read_text(
            encoding="utf-8"
        )
        cls.release = (ROOT / ".github" / "workflows" / "release.yml").read_text(
            encoding="utf-8"
        )

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
            '          python -m unittest discover -s scripts -p "test_check_release_workflow.py" -v\n',
            "",
        )
        self.assert_invalid(ci=changed, mentions="command list changed")

    def test_ci_primary_test_command_and_step_allowlist_are_pinned(self) -> None:
        removed = replace_once(
            self.ci, "        run: python test.py all", "        run: echo tests-skipped"
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
        optional = replace_once(self.release, "        required: true", "        required: false")
        self.assert_invalid(release=optional, mentions="required must be true")

        extra = replace_once(
            self.release,
            "options: [gore-save-editor, gore-mod-studio, gore-mod-manager, gore-cli]",
            "options: [gore-save-editor, gore-mod-studio, gore-mod-manager, gore-cli, surprise]",
        )
        self.assert_invalid(release=extra, mentions="options changed")

    def test_release_tag_triggers_permissions_and_signing_env_are_pinned(self) -> None:
        removed_tag = replace_once(
            self.release, '      - "gore-cli-v*"\n', ""
        )
        self.assert_invalid(release=removed_tag, mentions="tag set changed")

        changed_tag = replace_once(
            self.release, '      - "gore-mod-studio-v*"', '      - "gore-mod-studio-*"'
        )
        self.assert_invalid(release=changed_tag, mentions="tag set changed")

        unsigned = replace_once(self.release, '  GORE_SIGN: "1"', '  GORE_SIGN: "0"')
        self.assert_invalid(release=unsigned, mentions="release.env.GORE_SIGN: changed")

        readonly = replace_once(
            self.release, "permissions:\n  contents: write", "permissions:\n  contents: read"
        )
        self.assert_invalid(release=readonly, mentions="release.permissions.contents: changed")

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
