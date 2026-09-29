#!/usr/bin/env python3
"""Check instance independence and persisted-state recall from GUI test captures."""
import base64, json, math, pathlib, re
root = pathlib.Path(__file__).resolve().parents[2]/'evidence/editor-lifecycle'
def load(run, label):
    return json.loads((root/run/'artifacts'/f'{label}.json').read_text())
def params(states):
    return [json.loads(bytes(s['component']))['params'] for s in states]
baseline = load('foxcrush', 'baseline')
changed = load('foxcrush', 'edited')
a, b = params(changed)
assert a['bit_depth']['f32'] == 10 and b['bit_depth']['f32'] == 16
assert a['downsample']['f32'] == 1 and math.isclose(b['downsample']['f32'], 13, abs_tol=0.00001)
assert set(a) == set(b) == {'bit_depth', 'downsample', 'mix', 'output_gain'}
for run, label in [('foxcrush','recreated'), ('foxcrush','final'), ('foxcrush-recall','recall'), ('foxcrush-final','recall')]:
    restored = load(run, label)
    assert params(restored) == params(changed), (run, label)
    # Early captures queried before flushing controller feedback. Compare normalized
    # values with the post-recreation capture; component state above is authoritative.
    for before, after in zip(load('foxcrush', 'recreated'), restored):
        for expected, actual in zip(before['parameters'], after['parameters']):
            assert (expected['id'], expected['name']) == (actual['id'], actual['name'])
            assert math.isclose(expected['value'], actual['value'], abs_tol=1e-6), (run, label, expected, actual)
assert [p['id'] for p in baseline[0]['parameters']] == [p['id'] for p in changed[0]['parameters']]
# Decode the actual NIH-plug JSON saved inside REAPER's VST3 chunks.
rpp = (root/'reaper/artifacts/editor-regression.rpp').read_text()
states = []
for block in re.findall(r'<VST "VST3: Foxshaper[^\n]*\n(.*?)\n\s*>', rpp, re.S):
    # The first line is the host's bus metadata, followed by component state.
    lines = block.splitlines()[1:]
    raw = b''.join(base64.b64decode(line.strip()) for line in lines if line.strip())
    text = raw[raw.index(b'{"version"'):].decode('utf-8', errors='replace')
    state, _ = json.JSONDecoder().raw_decode(text)
    states.append(state['params'])
assert len(states) == 2
assert math.isclose(states[0]['depth']['f32'], .35, abs_tol=.00001)
assert states[1]['depth']['f32'] == 1
assert states[0]['shape']['f32'] == .5
assert math.isclose(states[1]['shape']['f32'], .79, abs_tol=.00001)
for run in ['foxcrush','foxcrush-recall','foxcrush-final','reaper','reaper-final']:
    result = json.loads((root/run/'result.json').read_text())
    assert result['exit'] == 0 and not result['timeout'], run
result = {'foxcrush_instance_values': params(changed), 'foxshaper_instance_values': states,
          'parameter_ids_preserved': True, 'fresh_process_and_recreation_recall': True,
          'clean_host_exits': True}
(root/'state-checks.json').write_text(json.dumps(result, indent=2)+'\n')
print('PASS: independent values, parameter IDs, recreation, fresh-process recall, saved REAPER states, clean exits')
