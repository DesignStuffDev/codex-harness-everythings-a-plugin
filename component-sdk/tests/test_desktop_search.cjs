// Request ownership tests; actual Chromium/engine interaction is a separate acceptance gate.
const {test} = require('node:test');
const assert = require('node:assert/strict');
const {SearchController, absoluteRoot, fileReference, insertReference} = require('../examples/desktop/desktop_ui/static/file-search.js');
const tick = () => new Promise(resolve => setImmediate(resolve));
const context = {root: '/project', threadId: 'thread-one', connection: 1};
const file = (path, root = '/project') => ({root, path, match_type: 'file', file_name: path, score: 100, indices: []});
function fixture() {
  let scheduled = null;
  const calls = [], states = [];
  const controller = new SearchController({
    pageId: 'unique-page',
    changed: state => states.push(state),
    schedule: callback => { scheduled = callback; return 1; },
    unschedule: () => { scheduled = null; },
    rpc: (method, params) => {
      let resolve, reject;
      const result = new Promise((ok, error) => { resolve = ok; reject = error; });
      calls.push({method, params, resolve, reject});
      if (!params.query) resolve({files: []});
      return result;
    },
  });
  return {controller, calls, states, flush() { const callback = scheduled; scheduled = null; callback?.(); }};
}
test('A to B to A fences the first A and bounds active requests to one plus latest', async () => {
  const f = fixture();
  f.controller.search('alpha', context); f.flush();
  for (let i = 0; i < 20; ++i) { f.controller.search(`other-${i}`, context); f.flush(); }
  f.controller.search('alpha', context); f.flush();
  assert.equal(f.calls.filter(call => call.params.query).length, 1);
  assert.equal(f.calls.filter(call => !call.params.query).length, 1);
  assert.equal(f.calls[0].params.cancellationToken, f.calls[1].params.cancellationToken);
  f.calls[0].resolve({files: [file('old-alpha')]}); await tick();
  assert.equal(f.calls.length, 3);
  assert.notEqual(f.calls[2].params.cancellationToken, f.calls[0].params.cancellationToken);
  assert.equal(f.states.some(state => state.files.some(result => result.path === 'old-alpha')), false);
  f.calls[2].resolve({files: [file('new-alpha')]}); await tick();
  assert.deepEqual(f.controller.state, {open: true, query: 'alpha', root: '/project', status: 'ready', files: [file('new-alpha')], error: ''});
});
test('same query after root and thread change cannot reuse earlier results', async () => {
  const f = fixture(); f.controller.search('same', context); f.flush();
  const next = {root: '/other', threadId: 'thread-two', connection: 2};
  f.controller.search('same', next); f.flush();
  f.calls[0].resolve({files: [file('wrong-root')]}); await tick();
  assert.deepEqual(f.calls[2].params.roots, ['/other']);
  f.calls[2].resolve({files: [file('right-root', '/other')]}); await tick();
  assert.deepEqual(f.controller.state.files, [file('right-root', '/other')]);
  assert.equal(f.states.some(state => state.files.some(result => result.path === 'wrong-root')), false);
});
test('close or reconnect fences late successes and errors without reopening', async () => {
  const f = fixture(); f.controller.search('file', context); f.flush(); f.controller.close(); f.controller.close();
  assert.equal(f.calls.length, 2);
  f.calls[0].reject(new Error('late failure')); await tick();
  assert.deepEqual(f.controller.state, {open: false, query: 'file', root: '/project', status: 'idle', files: [], error: ''});
  f.controller.search('file', {...context, connection: 2}); f.flush();
  f.calls[2].resolve({files: [file('after-reconnect')]}); await tick();
  assert.deepEqual(f.controller.state.files, [file('after-reconnect')]);
});
test('empty search cancels pending work and missing root never starts an ambient search', async () => {
  const f = fixture(); f.controller.search('file', context); f.flush(); f.controller.search('', context); f.flush();
  assert.equal(f.calls[1].params.query, '');
  f.calls[0].resolve({files: [file('late')]}); await tick();
  assert.equal(f.controller.state.status, 'idle'); assert.deepEqual(f.controller.state.files, []);
  f.controller.search('file', {...context, root: ''}); f.flush();
  assert.equal(f.calls.length, 2); assert.equal(f.controller.state.status, 'error');
});
test('backend errors and malformed or cross-root results stay explicit errors', async () => {
  const f = fixture(); f.controller.search('file', context); f.flush();
  f.calls[0].reject(new Error('selected search unavailable')); await tick();
  assert.equal(f.controller.state.error, 'selected search unavailable'); assert.equal(f.controller.state.status, 'error');
  for (const result of [{}, {files: [file('file', '/wrong')]}, {files: [file('../outside')]}]) {
    f.controller.search('file', context); f.flush(); f.calls.at(-1).resolve(result); await tick();
    assert.equal(f.controller.state.status, 'error'); assert.deepEqual(f.controller.state.files, []);
  }
});
test('path references consistently escape spaces, quotes, backslashes and preserve unsent text', () => {
  const special = file('docs/a "quoted" file.md');
  assert.equal(fileReference(special), '@"/project/docs/a \\"quoted\\" file.md"');
  assert.equal(fileReference(file('a b.rs', 'C:\\project')), '@"C:\\\\project\\\\a b.rs"');
  const insertion = insertReference('Please inspect this now.', 15, 19, special);
  assert.equal(insertion.text, 'Please inspect @"/project/docs/a \\"quoted\\" file.md" now.');
  assert.equal(insertion.text.slice(insertion.caret), ' now.');
});

test('relative roots fail before RPC and absolute roots follow the host platform', () => {
  const f = fixture();
  for (const root of ['.', '..', 'project', 'C:\\project']) {
    f.controller.search('file', {...context, root}); f.flush();
    assert.equal(f.controller.state.status, 'error');
    assert.match(f.controller.state.error, /absolute working directory/);
  }
  assert.equal(f.calls.length, 0);
  assert.equal(absoluteRoot('/project', 'posix'), true);
  for (const root of ['C:\\project', 'D:/project', '\\\\server\\share', '//server/share']) assert.equal(absoluteRoot(root, 'windows'), true);
  for (const root of ['C:project', '\\project', '/project', '//server', '.']) assert.equal(absoluteRoot(root, 'windows'), false);
});
