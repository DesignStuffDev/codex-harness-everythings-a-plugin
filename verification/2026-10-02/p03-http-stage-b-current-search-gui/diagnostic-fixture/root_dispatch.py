"""Root execution of the reviewed diagnostic retry; original failure is retained."""
from pathlib import Path
from collections import Counter
import hashlib
import json
import os
import subprocess
import time

D = Path(__file__).resolve().parent
C = Path('/workspace/codex-harness-everythings-a-plugin')
def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()
def read(path):
    return json.loads(Path(path).read_text())
def reference(row):
    assert Path(row['path']).stat().st_size == row['bytes']
    assert digest(row['path']) == row['sha256'], row['path']
assert digest(D / 'MANIFEST.json') == 'c83b3d9bc54cc1d575c9bc453296eb17e1d96aedc6a6e0d6c13bc9ac1ddc1dfd'
for row in read(D / 'MANIFEST.json')['files'].values():
    reference(row)
for row in read(D / 'INPUTS.json')['refs']:
    reference(row)
plan = read(D / 'COMMANDS.json')
binding = read(plan['postbuild_binding']['path'])
reference(plan['postbuild_binding'])
for name in ('cli', 'manager'):
    reference(binding[name])
source = read(binding['successful_build_source_receipt']['path'])
actual = {}
for scope in ('codex-rs', 'component-sdk', 'MODULE.bazel', 'MODULE.bazel.lock'):
    root = C / scope
    for path in sorted(root.rglob('*') if root.is_dir() else [root]):
        if any(part in {'target', '.git', '__pycache__'} for part in path.parts):
            continue
        if path.is_file() and not path.is_symlink():
            actual[str(path.relative_to(C))] = digest(path)
assert actual == source['source_before'] == source['source_after']
assert len(actual) == 8947
assert hashlib.sha256(json.dumps(actual, sort_keys=True, separators=(',', ':')).encode()).hexdigest() == plan['source_map_sha256']
MiB = 1024 * 1024
def resources():
    cg = Path('/sys/fs/cgroup')
    events = dict(line.split() for line in (cg / 'memory.events').read_text().splitlines())
    return {'memory_unused': int((cg / 'memory.max').read_text()) - int((cg / 'memory.current').read_text()), 'overlay_free': os.statvfs('/workspace').f_bavail * os.statvfs('/workspace').f_frsize, 'shm_free': os.statvfs('/dev/shm').f_bavail * os.statvfs('/dev/shm').f_frsize, 'oom': int(events['oom']), 'oom_kill': int(events['oom_kill'])}
initial = resources()
assert initial['memory_unused'] >= 900 * MiB and initial['overlay_free'] >= 128 * MiB and initial['shm_free'] >= 512 * MiB, initial
assert not (os.statvfs('/dev/shm').f_flag & os.ST_NOEXEC)
for row in plan['commands'].values():
    assert all(not Path(p).exists() for p in row['no_clobber_outputs'])
with (D / 'ROOT_PROGRESS.jsonl').open('x') as journal:
    for step in plan['sequence']:
        row = plan['commands'][step]
        before = resources()
        assert before['memory_unused'] >= 768 * MiB and before['oom'] == initial['oom'] and before['oom_kill'] == initial['oom_kill'], before
        if step == 'gui-search':
            previous = plan['commands']['migration-search']
            assert read(Path(previous['runtime_directory']) / 'manual-migration-report.json')['passed']
        print(json.dumps({'starting': step, 'resources': before}), flush=True)
        started = time.monotonic()
        result = subprocess.run(row['argv'], cwd=row['cwd'])
        after = resources()
        record = {'step': step, 'returncode': result.returncode, 'elapsed_seconds': time.monotonic() - started, 'resources_before': before, 'resources_after': after}
        sr = read(row['prefix'] + '.source.json')
        strict = read(row['prefix'] + '.strict.json')
        record.update(source_unchanged=sr['scoped_source_unchanged'], strict_exit=strict['exit_status'], strict_error=strict['runner_error'], adopted_statuses=dict(Counter(r['returncode'] for r in strict['reaped'])))
        assert sr['source_before'] == sr['source_after'] == actual
        if result.returncode == 0:
            assert sr['returncode'] == 0 and sr['scoped_source_unchanged']
            assert strict['subreaper'] and strict['command_returncode'] == strict['exit_status'] == 0 and strict['runner_error'] is None
            filename = 'manual-migration-report.json' if step == 'migration-search' else 'acceptance-private.json'
            assert read(Path(row['runtime_directory']) / filename)['passed']
        journal.write(json.dumps(record) + '\n')
        journal.flush()
        os.fsync(journal.fileno())
        print(json.dumps({'finished': record}), flush=True)
        if result.returncode != 0:
            raise SystemExit(result.returncode)
with (D / 'ROOT_COMPLETE.json').open('x') as stream:
    json.dump({'steps_returned_zero': plan['sequence'], 'original_search01_remains_failed': True, 'failure_cause_proven': False, 'in_app_browser_used': False, 'requires_observation_and_screenshot_review': True}, stream, indent=2)
    stream.write('\n')
