"""Assemble two independent role fixtures; never compile or write the shared tree.

The old A menu is hidden while every existing class and Act implementation stays
available to saved references. B is an exact original prefix plus new helpers.
"""
from pathlib import Path
import hashlib
import json
import re

REPO = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
BASE = HERE.parent / 'base'
OUT = REPO / 'work/npc-batch-tests/roles'
EXPECTED = {
    'A': 'b09651f45797f358e70d89c00e04aaff03a477d2cdc3b84151b1120846b192a1',
    'B': 'af5a3dbfd775e36643033c2b0d91f182c46836b7808bda67a3cd83115e04fcc0',
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def mask_literals(source):
    return re.sub(r'//[^\n]*|/\*.*?\*/|"(?:\\.|[^"\\])*"',
                  lambda m: ' ' * len(m[0]), source, flags=re.S)


def closing(masked, opening):
    depth = 1
    for i in range(opening + 1, len(masked)):
        depth += (masked[i] == '{') - (masked[i] == '}')
        if depth == 0:
            return i
    raise ValueError('Unbalanced baseline source')


def hide_old_choices(source):
    masked = mask_literals(source)
    edits, names = [], []
    for match in re.finditer(r'\bclass (UChoice\w+)\s*:\s*UTopic_Hero__GORE_TEST_A\s*\{', masked):
        start = masked.index('{', match.start())
        end = closing(masked, start)
        names.append(match[1])
        block = masked[start:end]
        visibility = re.search(r'\bbool\s+IsVisible(?:_Implementation)?\s*\([^)]*\)\s*const\s*\{', block)
        if visibility:
            body_start = start + block.index('{', visibility.start())
            body_end = closing(masked, body_start)
            edits.append((body_start + 1, body_end, '\n        return false;\n    '))
        else:
            edits.append((end, end, '    UFUNCTION(BlueprintOverride)\n    bool IsVisible() const { return false; }\n'))
    assert len(names) == 29, names
    for start, end, text in sorted(edits, reverse=True):
        source = source[:start] + text + source[end:]
    return source, names


def split_fragment(name):
    text = (HERE / name).read_text(encoding='utf-8')
    marker = 'namespace G1R::Conversation'
    assert text.count(marker) == 1
    before, after = text.split(marker)
    return before, marker + after


def main():
    baseline = {key: (BASE / f'GORE_TEST_{key}.as').read_bytes() for key in EXPECTED}
    assert {key: digest(value) for key, value in baseline.items()} == EXPECTED
    old_a = baseline['A'].decode('utf-8')
    hidden_a, names = hide_old_choices(old_a)
    common_b, common_a = split_fragment('common.as')
    report = {'baseline_sha256': EXPECTED, 'hidden_original_choices': names, 'variants': {}}
    for variant in ('economy', 'field'):
        helper, menu = split_fragment(f'{variant}.as')
        a = (hidden_a + '\n' + common_a + '\n' + menu).encode('utf-8')
        b = baseline['B'] + ('\n' + common_b + '\n' + helper).encode('utf-8')
        target = OUT / variant
        target.mkdir(parents=True, exist_ok=True)
        # Source staging is deliberately distinct from root's shared compile tree.
        (target / 'GORE_TEST_A.as').write_bytes(a)
        (target / 'GORE_TEST_B.as').write_bytes(b)
        classes_before = re.findall(r'\bclass\s+(\w+)', old_a)
        classes_after = re.findall(r'\bclass\s+(\w+)', a.decode())
        assert all(name in classes_after for name in classes_before)
        assert b.startswith(baseline['B'])
        # Brace/unique class and DebugId checks are assembly checks, not compilation.
        joined = a.decode() + '\n' + b.decode()
        masked = mask_literals(joined)
        assert masked.count('{') == masked.count('}')
        classes = re.findall(r'\bclass\s+(\w+)', masked)
        assert len(classes) == len(set(classes))
        ids = re.findall(r'default DebugId = (\d+);', joined)
        assert len(ids) == len(set(ids))
        item = {
            'A_sha256': digest(a), 'B_sha256': digest(b),
            'all_original_A_classes_retained': True,
            'original_A_choice_visibility_false_count': len(names),
            'original_B_prefix_byte_exact': True,
            'new_choices_in_A': True, 'helper_classes_in_B': True,
            'source_checks_passed': True,
            'compiler_admission': 'pending root build', 'runtime': 'not tested',
        }
        (target / 'source-evidence.json').write_text(json.dumps(item, indent=2) + '\n', encoding='utf-8')
        report['variants'][variant] = item
    (OUT / 'source-evidence.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(report['variants'], indent=2))


if __name__ == '__main__':
    main()
