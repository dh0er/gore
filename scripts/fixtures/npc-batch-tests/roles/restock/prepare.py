"""Assemble the late-stock follow-up, retaining all saved economy classes.

Writes only this fixture's two tracked source files and source manifest.
No game, archive, or live save files are touched.
"""
from pathlib import Path
import hashlib
import json
import re

HERE = Path(__file__).resolve().parent


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    inputs = {key: (HERE.parent / 'economy' / f'GORE_TEST_{key}.as').read_bytes()
              for key in ('A', 'B')}
    a, b = (inputs[key].decode('utf-8').replace('\r\n', '\n') for key in ('A', 'B'))
    # Hide completed teacher/initial-stock menus, but retain their saved types.
    for name in ('Stock', 'Learn', 'Funds', 'EconomyRead'):
        pattern = (r'(class UChoiceGoreRole' + name + r'\b.*?'
                   r'bool IsVisible\(\) const \{)[^}]+(\})')
        a, count = re.subn(pattern, r'\1 return false; \2', a, flags=re.S)
        assert count == 1, name
    old = '02 Handel: kaufen / verkaufen'
    assert a.count(old) == 1
    a = a.replace(old, '01 Handel: Bestand ansehen')
    old = 'return GoreRoleRead(this.GetSelf(), n"gore_role_shop_stock_v2") == 1.0f;'
    assert a.count(old) == 1
    a = a.replace(old, 'return this.GetSelf() != nullptr && Hero() != nullptr;')
    a += '\n' + (HERE / 'restock.as').read_text(encoding='utf-8')
    marker = '    default AddTraderItemAllDifficulties(UItMi_Orenugget, 100, "OnWorldStart");'
    assert b.count(marker) == 1
    b = b.replace(marker, marker + '\n'
        '    // Registered alongside the initial batch, dispatched later by choice02.\n'
        '    default AddTraderItemAllDifficulties(UItFo_Cheese, 2, "GoreNpcRestockTrialV1");\n'
        '    default AddTraderItemAllDifficulties(UItAm_Arrow, 5, "GoreNpcRestockTrialV1");\n'
        '    default AddTraderItemAllDifficulties(UItMi_Orenugget, 20, "GoreNpcRestockTrialV1");')
    outputs = {'A': a.encode('utf-8'), 'B': b.encode('utf-8')}
    for key, data in outputs.items():
        (HERE / f'GORE_TEST_{key}.as').write_bytes(data)
        old_classes = set(re.findall(rb'\bclass\s+(\w+)', inputs[key]))
        assert old_classes <= set(re.findall(rb'\bclass\s+(\w+)', data))
    ids = re.findall(r'default DebugId = (\d+);', a + '\n' + b)
    assert len(ids) == len(set(ids))
    report = {
        'schema': 'gore.npc-restock.sources.v1',
        'parent': 'NpcEconomyRolesTest 0.1.3',
        'input_sha256': {k: sha(v) for k, v in inputs.items()},
        'output_sha256': {k: sha(v) for k, v in outputs.items()},
        'all_existing_classes_retained': True,
        'initial_event': 'OnWorldStart', 'late_event': 'GoreNpcRestockTrialV1',
        'initial_stock': {'cheese': 3, 'arrows': 10, 'ore': 100},
        'late_batch': {'cheese': 2, 'arrows': 5, 'ore': 20},
        'runtime': 'pending user test',
    }
    (HERE / 'sources.json').write_text(json.dumps(report, indent=2) + '\n',
                                      encoding='utf-8', newline='\n')
    print('Restock fixture assembled; old classes retained, only three test choices plus End visible.')


if __name__ == '__main__':
    main()
