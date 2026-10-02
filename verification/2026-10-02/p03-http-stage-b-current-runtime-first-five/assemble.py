"""Read-only, public-safe projections of completed receipts. Never opens an ELF or raw trace."""
import collections
import datetime
import hashlib
import json
from pathlib import Path

R = Path('/workspace/recovery-backups/20260930T165936Z')
A = Path('/workspace/acceptance')
OUT = Path(__file__).resolve().parent
references = []

def read(path, role):
    path = Path(path)
    assert path.stat().st_size < 8_000_000, 'unexpected evidence size'
    data = path.read_bytes()
    references.append({'role': role, 'path': str(path), 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()})
    return json.loads(data)

def fp(v):
    return {'bytes': v['bytes'], 'sha256': v['sha256']}

def histogram(records):
    return dict(sorted(collections.Counter(str(x['returncode']) for x in records).items()))

def write(name, obj):
    (OUT / name).write_text(json.dumps(obj, indent=2, sort_keys=True) + '\n')

build = read(R / 'p03-http-stage-b-production-build-evidence-01/REPORT.json', 'current production build summary')
source = {k: build['source'][k] for k in ['count', 'map_sha256', 'candidate_manifest_sha256', 'source_publication_commit', 'source_publication_tree']}
assert source['count'] == 8947
assert source['map_sha256'] == '7a147c1d2929659d92eea648a4411e746c2726712a7f3b5485f332de83bacd4e'
expected = {'codex': fp(build['outputs']['new_cli']), 'host': fp(build['outputs']['manager'])}
assert expected['codex']['sha256'] == '78d9194cd4329e9b83b75fe07389d524d8b1f425353d71df9a68607be2945e83'
assert expected['host']['sha256'] == 'f054d84acba3ea6edb6c20f08a037a087dc324fb953ca29f9649f1ab8f473954'

def host_pair(before, after, attachment=False):
    if attachment:
        before = {'codex': before['CLI'], 'host': before['manager']}
        after = {'codex': after['CLI'], 'host': after['manager']}
    assert {k: fp(v) for k, v in before.items()} == expected
    assert {k: fp(v) for k, v in after.items()} == expected
    return True

def receipt(gate):
    prefix = A / f'p03-http-stage-b-newhost-{gate}-01'
    d = read(str(prefix) + '.source.json', gate + ' source receipt')
    s = read(str(prefix) + '.strict.json', gate + ' strict receipt')
    assert d['status'] == 'finished' and d['returncode'] == 0 and d['scoped_source_unchanged'] is True
    assert d['source_before'] == d['source_after'] == d['candidate_manifest']['source_map']
    assert len(d['source_before']) == source['count']
    assert d['candidate_manifest']['source_map_sha256'] == source['map_sha256']
    assert d['candidate_manifest_sha256'] == source['candidate_manifest_sha256']
    assert s['command_returncode'] == s['exit_status'] == 0 and s['runner_error'] is None
    return {'source_unchanged': True, 'before_after_equal_candidate': True, 'source_count': len(d['source_before']),
            'source_wrapper_status': d['returncode'], 'started_utc': d['started_utc'], 'finished_utc': d['finished_utc'],
            'log_sha256': d['log_sha256'], 'strict': {'command_returncode': s['command_returncode'], 'exit_status': s['exit_status'],
            'runner_error': s['runner_error'], 'subreaper': s['subreaper'], 'adopted_status_histogram': histogram(s['reaped']),
            'elapsed_seconds': s['elapsed_seconds']}}

gates = {}
for gate in ['storage', 'migration-normal', 'gui-normal', 'attachment', 'migration-search']:
    gates[gate] = {'receipt': receipt(gate)}

storage = read('/tmp/p03-http-stage-b-newhost-storage-01-runtime/independent-report.json', 'storage reuse public report')
runtime = read(storage['runtime_report'], 'storage runtime private report (safe projection only)')
commands = read(runtime['commands_file'], 'storage commands private report (statuses only)')
assert storage['passed'] and runtime['passed'] and not storage['integrity_errors']
assert storage['evidence_kind'] == 'reused_package_new_host' and storage['new_independent_build'] is False
host_pair(storage['binaries_before'], storage['binaries_after'])
host_pair(runtime['binaries_before'], runtime['binaries_after'])
assert storage['package_files'] == storage['package_files_after']
assert all(c['status'] == 0 for c in commands)
assert all(c['terminated'] for c in runtime['storage_children'])
gates['storage'].update({
    'passed': True, 'runtime_command_count': len(commands), 'runtime_command_statuses': [c['status'] for c in commands],
    'evidence_kind': storage['evidence_kind'], 'new_independent_build': False,
    'contract_version': storage['contract_version'], 'package_version': storage['package_version'],
    'source_build_report_fingerprint': fp(storage['source_build_report_fingerprint']),
    'package_files': {k: fp(v) for k, v in storage['package_files'].items()},
    'original_export_absent_before_and_after': storage['source_removed_before_install'] and storage['source_export_absent_after'],
    'host_and_package_unchanged': True,
    'operations': ['Install and select unchanged independent native thread-store package against rebuilt host.',
                   'Real CLI turn, external calculator execution/context contribution and model-event streaming.',
                   'Cold CLI session resume with retained tool and assistant history.',
                   'No native model requests while replacement model selected; reset model restores loopback native transport.',
                   'Reset thread_store selection restores built-in storage with retained history.'],
    'observed_storage_children': len(runtime['storage_children']), 'observed_storage_children_terminated': True,
    'limits': ['Native package was reused from its earlier independent build; Python fixture plugins were built in this run.',
               'Inference is deterministic external/loopback fixture, not a live provider.',
               'Storage watcher checks observed installed executables disappear; it is not a universal descendant or graceful-exit proof.',
               'Outer strict receipt includes SIGKILL (-9) descendant statuses; no cause is attributed here.',
               'Storage reset is proven; package uninstall is not part of this storage gate.']})

for gate in ['migration-normal', 'migration-search']:
    d = read(f'/tmp/p03-http-stage-b-newhost-{gate}-01-runtime/manual-migration-report.json', gate + ' private report (safe projection only)')
    assert d['passed'] and d['host_unchanged'] and all(c['status'] == 0 for c in d['commands'])
    host_pair(d['binaries_before'], d['binaries_after'])
    gates[gate].update({'passed': True, 'command_count': len(d['commands']), 'commands': [
        {'label': c['label'], 'status': c['status'], 'selected_worker_observed_count': len(c['storage_pids'])} for c in d['commands']],
        'host_unchanged': True, 'verified': d['verified'], 'observed_installed_storage_processes': len(d['storage_processes']),
        'observed_storage_process_absence_asserted': True,
        'limits': ['Deterministic fixture, not live inference.', 'This migration gate does not run browser checks; the later GUI gate is separate.',
                   'Passing migration-search does not establish selected-search GUI success.',
                   'Outer strict receipt includes SIGKILL (-9) descendant statuses; no cause is attributed here.']})

gui = read('/tmp/p03-http-stage-b-newhost-gui-normal-01-runtime/acceptance-private.json', 'GUI-normal private report (safe projection only)')
assert gui['passed'] and len(gui['runs']) == 2 and gui['installed_packages_unchanged']
host_pair(gui['binaries_before'], gui['binaries_after'])
assert gui['subreaper']['passed'] and gui['final_drain']['passed']
gui_runs = []
for run in gui['runs']:
    assert run['passed'] and run['first_sigint'] and run['manager_status'] == 0 and run['tracked_processes_absent'] and run['subreaper_drain']['passed']
    search = run['file_search']
    assert search['passed']
    gui_runs.append({'cycle': run['cycle'], 'passed': run['passed'], 'first_sigint': run['first_sigint'],
        'manager_status': run['manager_status'], 'manager_exit_wait_seconds': run['shutdown_seconds'],
        'tracked_processes_absent': run['tracked_processes_absent'], 'subreaper_drain_passed': run['subreaper_drain']['passed'],
        'subreaper_adopted_status_histogram': histogram(run['subreaper_drain']['reaped']),
        'installed_storage_fingerprint': fp(run['installed_storage_fingerprint']),
        'file_search': {'passed': search['passed'], 'backend': search['backend'], 'verified': search['verified'], 'response_barriers': search['response_barriers']}})
gates['gui-normal'].update({'passed': True, 'browser': gui['browser'], 'inference': gui['inference'], 'cycles': gui_runs,
    'host_and_installed_packages_unchanged': True, 'combined_subreaper_passed': gui['subreaper']['passed'],
    'final_drain_passed': gui['final_drain']['passed'], 'tracking_scope': gui['tracking_scope'],
    'script_fingerprint': fp(gui['script_fingerprint']), 'driver_fingerprint': fp(gui['driver_fingerprint']),
    'behavior': ['Cycle 1: real streamed turn, approval and allowed native command, UI Stop, browser reload and continuation.',
                 'Cycle 2: cold engine restart recovers persisted conversation and completes a continued streamed turn.',
                 'Both cycles: real native file-search UI behavior and first-SIGINT component-manager Launch shutdown with tracked process absence.'],
    'limits': ['Playwright Chromium fallback, not manual in-app Browser; no live model provider.',
               'Approval and UI Stop are cycle-1 checks, not assertions duplicated in cycle 2.',
               'This gate uses built-in fuzzyFileSearch; it does not validate installed file-search replacement.',
               'Manager exit-wait duration is not the entire test or general shutdown budget.',
               'Observed descendants and subreaper drains are scoped; short-lived descendants may evade scans.',
               'This summary did not review or export private screenshot contents.',
               'Slow-storage deadline, forced termination, held constructor/transport shutdown and universal host cleanliness are separate gates.']})

attachment = read('/tmp/p03-http-stage-b-newhost-attachment-01-runtime/acceptance.json', 'attachment private report (safe projection only)')
assert attachment['passed'] and attachment['host_unchanged'] and attachment['native_package_unchanged'] and attachment['installed_packages_unchanged']
host_pair(attachment['host_before'], attachment['host_after'], attachment=True)
assert len(attachment['commands']) == 17 and len(attachment['cases']) == 5
command_projection = []
for c in attachment['commands']:
    assert c['passed'] and c['cleanup_confirmed'] and c['tracked_processes_absent'] and not c['forced_fixture_cleanup']
    assert c['command_returncode'] == c['strict_exit_status'] == c['wrapper_returncode'] == 0 and c['strict_runner_error'] is None
    command_projection.append({k: c[k] for k in ['label', 'passed', 'cleanup_confirmed', 'tracked_processes_absent', 'forced_fixture_cleanup',
        'command_returncode', 'strict_exit_status', 'wrapper_returncode', 'strict_runner_error', 'adopted_status_histogram', 'fallback_warning_count']})
case_projection = {}
for name, case in attachment['cases'].items():
    assert case['passed']
    case_projection[name] = {k: v for k, v in case.items() if k not in ['model_image', 'trace', 'proof']}
trace = attachment['cases']['native_inline']['trace']
trace_projection = {k: trace[k] for k in ['native_exec_count', 'native_leader_exit_zero', 'native_result_envelope_proven',
    'native_upload_path_invoked', 'owned_thread_count', 'staged_read_open_count', 'whole_host_clean']}
gates['attachment'].update({'passed': True, 'cases': case_projection, 'command_count': len(command_projection), 'commands': command_projection,
    'native_upload_path_observation': trace_projection,
    'native_package_build': attachment['native_package_build'], 'native_package_files': attachment['native_package_before'],
    'native_package_unchanged': attachment['native_package_unchanged'], 'native_independent_report_sha256': attachment['native_report_sha256'],
    'initial_objects_preserved_through_native_install': attachment['initial_objects_preserved_through_native_install'],
    'native_removal': attachment['native_removal'], 'host_and_installed_packages_unchanged': True,
    'resolve_production_caller_exercised': attachment['resolve_production_caller_exercised'],
    'native_result_envelope_inspected': attachment['native_result_envelope_inspected'], 'whole_host_clean': attachment['whole_host_clean'],
    'limits': ['Historical independently built native inline package reused; no new native package compilation.',
               'Real native upload execution is supported by the trace inspector report; this assembly does not reread raw private traces.',
               'No production resolve caller or inspected native result envelope proof.',
               'Native state directory retained with zero files; this does not demonstrate nonempty native state preservation.',
               'Nested adopted SIGKILL statuses are retained; no causal attribution or universal clean-host claim.',
               'Model responses are deterministic installed fixture output, not live inference.']})

failed_source = read(A / 'p03-http-stage-b-newhost-gui-search-01.source.json', 'excluded failed selected-search GUI source receipt')
failed_strict = read(A / 'p03-http-stage-b-newhost-gui-search-01.strict.json', 'excluded failed selected-search GUI strict receipt')
assert failed_source['returncode'] == failed_strict['command_returncode'] == failed_strict['exit_status'] == 1
report = {'schema': 'current-host-passed-gates-public-projection-v1', 'assembled_utc': datetime.datetime.now(datetime.timezone.utc).isoformat(),
    'scope': 'Five completed passing gates only; read-only review, no rerun, no source edit, no new ELF hashing.',
    'source': source, 'binaries': expected, 'gate_count': 5, 'gates': gates,
    'excluded_failed_gate': {'name': 'gui-search attempt01', 'source_wrapper_status': failed_source['returncode'],
        'strict_command_status': failed_strict['command_returncode'], 'strict_exit_status': failed_strict['exit_status'],
        'strict_runner_error': failed_strict['runner_error'], 'scoped_source_unchanged': failed_source['scoped_source_unchanged'],
        'diagnosis': 'Not performed by this review; retained as failure. Its partial cycles are not counted among the five passing gates.'},
    'remaining_acceptance': ['Current-host selected-search GUI pass remains absent from this evidence subset.',
        'Slow normal/forced storage shutdown, held Git/HTTP matrix and featured-owner current-host gates must be joined from their own receipts.',
        'CLI/exec/TUI focused package gaps, held constructor deadlines, lower transport shutdown and whole-host hard-exit must not be inferred from successful build or these gates.',
        'No live-provider or manual in-app Browser evidence.',
        'Auth reload/refresh proposals remain unadopted and are not exercised by this host.',
        'No additional extracted component family or completed P03/whole-platform/updater acceptance is claimed.'],
    'privacy': 'Only whitelisted assertions, counts, command labels and fingerprints. No thread IDs, prompts, payloads, auth/config values, raw traces, private screenshot contents or raw command arguments copied.',
    'durability': 'Staged only in the original cloud filesystem; external durability requires root publication/readback.',
    'method_limits': ['The report verifies agreement between preserved source/strict/runtime receipts, not a new runtime execution.',
        'Binary hashes come from root terminal/build/runtime receipts; no ELF was opened by this assembly.',
        'Publication identity is copied from the build evidence; this assembly did not query GitHub.']}
write('REPORT.json', report)
write('LOCAL_REFERENCES.json', {'schema': 'local-reference-fingerprints-v1', 'references': references})
print(json.dumps({'gate_count': len(gates), 'report_bytes': (OUT/'REPORT.json').stat().st_size, 'references': len(references)}))
