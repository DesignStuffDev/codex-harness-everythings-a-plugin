'use strict';
// Observation only: never route, retry, alter, cancel, or fulfill an application request.
const fs = require('node:fs');
const path = require('node:path');
const LIMIT = Object.freeze({events: 256, requests: 128, pendingBodies: 8, bodyBytes: 65536, snapshots: 4, failureBudgetMs: 3000});
const expectedName = 'p02 beta "quoted"\\notes.md';
function category(text) {
  if (typeof text !== 'string') return 'none';
  if (!text) return 'none';
  if (/absolute working directory/i.test(text)) return 'relative_root';
  if (/disconnected|timed out|timeout/i.test(text)) return 'disconnected_or_timeout';
  if (/invalid file search result/i.test(text)) return 'invalid_search_result';
  if (/no space left/i.test(text)) return 'no_space';
  if (/closed/i.test(text)) return 'closed';
  if (/denied|permission/i.test(text)) return 'permission';
  return 'other';
}
function queryKind(text) {
  return new Map([['p02 beta', 'click'], ['p02-alpha', 'keyboard'], ['p02-root', 'root']]).get(text) || (text === '' ? 'empty' : 'other');
}
function boundedCount(value, limit = 1000000) {
  return Number.isSafeInteger(value) && value >= 0 ? Math.min(value, limit) : null;
}
function install(page, reportFile) {
  const output = path.join(path.dirname(reportFile), path.basename(reportFile, '.json') + '.search-observation.json');
  const report = {schema: 'bounded-gui-search-observation-v1', diagnosticOnly: true, limits: LIMIT, events: [], snapshots: [], droppedEvents: 0, ignoredRequests: 0, skippedBodies: 0, collectorFailures: 0, finished: false};
  const requests = new WeakMap();
  let ordinal = 0, pendingBodies = 0, snapshots = 0;
  const increment = key => { report[key] = Math.min(1000000, report[key] + 1); };
  const save = () => { try { fs.writeFileSync(output, JSON.stringify(report, null, 2) + '\n', {mode: 0o600}); } catch { increment('collectorFailures'); } };
  const record = event => {
    if (report.events.length < LIMIT.events) report.events.push({elapsedMs: Math.round(performance.now()), ...event});
    else increment('droppedEvents');
    save();
  };
  const safe = callback => (...args) => { try { callback(...args); } catch { increment('collectorFailures'); save(); } };
  page.on('request', safe(request => {
    // Do not inspect URL/query string, request headers, bearer values, or unrelated payloads.
    if (request.method() !== 'POST') return;
    const buffer = request.postDataBuffer();
    if (!buffer || buffer.length > LIMIT.bodyBytes) return;
    let frame; try { frame = JSON.parse(buffer.toString('utf8')); } catch { return; }
    if (frame?.method !== 'fuzzyFileSearch') return;
    if (ordinal >= LIMIT.requests) { increment('ignoredRequests'); save(); return; }
    const id = ++ordinal;
    requests.set(request, id);
    const params = frame.params || {};
    record({kind: 'request', id, query: queryKind(params.query), queryBytes: typeof params.query === 'string' ? Math.min(LIMIT.bodyBytes, Buffer.byteLength(params.query)) : null, rootsCount: Array.isArray(params.roots) ? boundedCount(params.roots.length) : null, firstRootAbsolute: Array.isArray(params.roots) && typeof params.roots[0] === 'string' && params.roots[0].startsWith('/'), cancellationTokenPresent: typeof params.cancellationToken === 'string'});
  }));
  page.on('requestfinished', safe(request => {
    const id = requests.get(request); if (id !== undefined) record({kind: 'request_finished', id});
  }));
  page.on('requestfailed', safe(request => {
    const id = requests.get(request); if (id !== undefined) record({kind: 'request_failed', id, failure: category(request.failure()?.errorText)});
  }));
  page.on('response', safe(response => {
    const request = response.request(), id = requests.get(request);
    if (id === undefined) return;
    record({kind: 'response_headers', id, status: response.status()});
    if (pendingBodies >= LIMIT.pendingBodies) { increment('skippedBodies'); save(); return; }
    pendingBodies += 1;
    (async () => {
      const failed = await response.finished();
      if (failed) { record({kind: 'response_body_unavailable', id, reason: 'response_not_finished_cleanly'}); return; }
      // Gateway replies have no Content-Length. Request.sizes() provides the completed wire-body size.
      // Reject encoded replies and oversized/unknown sizes before requesting a decoded body from Playwright.
      const encoding = await response.headerValue('content-encoding');
      const sizes = await request.sizes();
      const bytes = sizes.responseBodySize;
      if ((encoding && encoding !== 'identity') || !Number.isSafeInteger(bytes) || bytes < 0 || bytes > LIMIT.bodyBytes) {
        record({kind: 'response_body_unavailable', id, reason: 'encoded_or_unbounded_body'}); return;
      }
      const body = await response.body();
      if (body.length > LIMIT.bodyBytes) { record({kind: 'response_body_unavailable', id, reason: 'body_overflow'}); return; }
      let value; try { value = JSON.parse(body.toString('utf8')); } catch { record({kind: 'response_body_unavailable', id, reason: 'non_json'}); return; }
      const files = value?.result?.files;
      record({kind: 'response_summary', id, bodyBytes: body.length, rpcErrorPresent: value?.error != null, rpcErrorCode: Number.isSafeInteger(value?.error?.code) ? value.error.code : null, error: category(value?.error?.message), filesIsArray: Array.isArray(files), filesCount: Array.isArray(files) ? boundedCount(files.length) : null, expectedFixtureFilePresent: Array.isArray(files) && files.some(file => file?.path === expectedName)});
    })().catch(() => { record({kind: 'response_body_unavailable', id, reason: 'collection_failed'}); }).finally(() => { pendingBodies -= 1; });
  }));
  async function onCommandFailure(commandOrdinal) {
    if (snapshots >= LIMIT.snapshots) return;
    const snapshotOrdinal = ++snapshots;
    const snapshot = {commandOrdinal, stateCollected: false, screenshotCollected: false};
    report.snapshots.push(snapshot);
    let timer;
    const collect = (async () => {
      try {
        snapshot.dom = await page.evaluate(() => {
          const el = id => document.getElementById(id);
          const statusText = el('file-search-status')?.textContent || '';
          const query = el('file-query')?.value || '';
          const root = el('cwd')?.value || '';
          const error = el('file-search-error')?.textContent || '';
          let errorKind = 'none';
          if (error) errorKind = /absolute working directory/i.test(error) ? 'relative_root' : /disconnected|timed out|timeout/i.test(error) ? 'disconnected_or_timeout' : /invalid file search result/i.test(error) ? 'invalid_search_result' : /no space left/i.test(error) ? 'no_space' : /closed/i.test(error) ? 'closed' : /denied|permission/i.test(error) ? 'permission' : 'other';
          return {pickerOpen: Boolean(el('file-picker') && !el('file-picker').hidden), connected: Boolean(el('connection')?.classList.contains('connected')), queryIsExpected: query === 'p02 beta', absoluteRoot: root.startsWith('/'), displayedRootMatchesInput: el('file-root')?.textContent === root.trim(), status: statusText === 'Searching…' ? 'loading' : statusText === 'No matching files' ? 'empty' : statusText === 'Type a file name' ? 'idle' : /results · Enter to insert a reference$/.test(statusText) ? 'results' : 'other', errorPresent: Boolean(error), errorVisible: Boolean(el('file-search-error') && !el('file-search-error').hidden), errorKind, resultCount: Math.min(1000000, document.querySelectorAll('#file-results button').length)};
        });
        snapshot.stateCollected = true;
        const box = await page.locator('#file-picker').boundingBox({timeout: 750});
        const viewport = page.viewportSize();
        if (box && viewport && box.width > 0 && box.height > 0) {
          const x = Math.max(0, box.x), y = Math.max(0, box.y);
          const width = Math.min(box.x + box.width, viewport.width) - x, height = Math.min(box.y + box.height, viewport.height) - y;
          if (width > 0 && height > 0) {
            const name = path.basename(reportFile, '.json') + '.search-failure-' + snapshotOrdinal + '.png';
            await page.screenshot({path: path.join(path.dirname(reportFile), name), clip: {x, y, width, height}, timeout: 1250});
            snapshot.screenshotCollected = true; snapshot.screenshot = name;
          }
        }
      } catch { snapshot.collectionFailed = true; }
      save();
    })();
    try { await Promise.race([collect, new Promise(resolve => { timer = setTimeout(() => { snapshot.budgetExpired = true; resolve(); }, LIMIT.failureBudgetMs); })]); }
    catch { snapshot.collectionFailed = true; }
    finally { clearTimeout(timer); save(); }
  }
  save();
  return {onCommandFailure, finish() { report.finished = true; report.finishedMeaning = 'close_requested_not_all_collectors_completed'; report.pendingBodiesAtClose = pendingBodies; save(); }};
}
module.exports = {install};
