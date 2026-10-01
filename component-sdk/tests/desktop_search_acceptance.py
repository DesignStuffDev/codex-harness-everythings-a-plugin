"""Opt-in real desktop search interaction against existing App Server RPCs.

Response barriers delay actual route.fetch replies without fabricating search
results. All files belong to the fresh acceptance fixture, never a prior home.
"""

import json
from pathlib import Path


def prepare(project, work):
    alternate = work / "alternate-search-root"
    alternate.mkdir()
    names = {
        "click": 'p02 beta "quoted"\\notes.md',
        "keyboard": "p02-alpha.txt",
        "original": "p02-root-original.txt",
        "alternate": "p02-root-alternate.txt",
    }
    for key, name in names.items():
        root = alternate if key == "alternate" else project
        (root / name).write_text(f"Real native search fixture: {key}\n")
    return {"root": str(project), "alternate_root": str(alternate), **names}


def hold_real_reply(browser, query, root):
    browser.command(
        "globalThis.p02Gate = {claimed:false, held:false, searches:[]}; "
        "p02Gate.ready = new Promise(resolve => {p02Gate.release = resolve;}); "
        f"const wanted = {json.dumps(query)}, root = {json.dumps(root)}; "
        "await page.route('**/rpc', async route => {const frame = route.request().postDataJSON(); "
        "if (frame.method !== 'fuzzyFileSearch') {await route.continue(); return;} "
        "p02Gate.searches.push(frame.params); "
        "if (!p02Gate.claimed && frame.params.query === wanted && frame.params.roots[0] === root) {"
        "p02Gate.claimed = true; const response = await route.fetch(); const body = await response.body(); "
        "p02Gate.held = true; await p02Gate.ready; await route.fulfill({response, body});"
        "} else await route.continue();}); "
        f"await page.locator('#file-query').fill({json.dumps(query)}); "
        "for (let i=0; !p02Gate.held && i<600; i++) await new Promise(resolve => setTimeout(resolve, 50)); "
        "if (!p02Gate.held) throw new Error('Real native response was not observed at barrier'); "
        "return {realResponseHeld:true};"
    )


def release_reply(browser):
    browser.command("p02Gate.release(); return {released:true};")


def wait_result(browser, name):
    browser.command(
        f"const name = {json.dumps(name)}; "
        "await page.waitForFunction(name => [...document.querySelectorAll('#file-results button span')].some(n => n.textContent === name), name); "
        "if (await page.locator('#file-search-error').isVisible()) throw new Error('Visible file-search error'); return {result:true};"
    )


def exercise(browser, fixture, cycle, thread_id, work, screenshot):
    root = fixture["root"]
    prefix = f"Review these real file references after restart {cycle}: "
    expected = (
        prefix
        + "@"
        + json.dumps(str(Path(root) / fixture["click"]), ensure_ascii=False)
        + " "
    )
    before = browser.command(
        f"await page.locator('#prompt').fill({json.dumps(prefix)}); "
        "await page.locator('#prompt').evaluate(n => n.setSelectionRange(n.value.length,n.value.length)); "
        "globalThis.p02RelativeRequests = []; globalThis.p02RelativeObserver = request => {if (request.url().endsWith('/rpc') && request.postDataJSON()?.method === 'fuzzyFileSearch') p02RelativeRequests.push(request.postDataJSON());}; page.on('request', p02RelativeObserver); "
        "await page.locator('#cwd').fill('.'); await page.locator('#find-file').click(); "
        "await page.locator('#file-query').fill('p02 beta'); "
        "return await page.locator('#messages article').count();"
    )
    browser.command(
        "await page.waitForFunction(() => document.querySelector('#file-search-error').textContent.includes('absolute working directory')); await page.waitForTimeout(250); "
        "if (p02RelativeRequests.length || await page.locator('#cwd').inputValue() !== '.') throw new Error('Relative root was searched or silently changed'); "
        f"if (await page.locator('#prompt').inputValue() !== {json.dumps(prefix)}) throw new Error('Relative-root failure changed draft'); "
        "page.off('request', p02RelativeObserver); "
        f"await page.locator('#cwd').fill({json.dumps(root)}); return {{relativeRootRejected:true}};"
    )
    wait_result(browser, fixture["click"])
    screenshot(browser, work / f"search-results-{cycle}.png")
    browser.command(
        f"await page.locator('#file-results button').filter({{hasText:{json.dumps(fixture['click'])}}}).click(); "
        f"if (await page.locator('#prompt').inputValue() !== {json.dumps(expected)}) throw new Error('Click insertion did not preserve draft/quoted path'); "
        "if (await page.locator('#file-picker').isVisible()) throw new Error('Picker did not close'); return {clickInsertion:true};"
    )
    browser.command(
        "await page.locator('#find-file').click(); await page.locator('#file-query').fill('p02-alpha'); return {keyboardSearch:true};"
    )
    wait_result(browser, fixture["keyboard"])
    expected += (
        "@"
        + json.dumps(str(Path(root) / fixture["keyboard"]), ensure_ascii=False)
        + " "
    )
    browser.command(
        "await page.locator('#file-query').press('ArrowDown'); await page.locator('#file-query').press('ArrowUp'); await page.locator('#file-query').press('Enter'); "
        f"if (await page.locator('#prompt').inputValue() !== {json.dumps(expected)}) throw new Error('Keyboard insertion changed draft'); "
        f"if (await page.locator('#messages article').count() !== {before}) throw new Error('Enter submitted a turn instead of choosing a file'); "
        "return {keyboardInsertion:true};"
    )
    # Start A, hold its real result, then exercise B -> A with distinct query IDs.
    browser.command(
        "await page.locator('#find-file').click(); await page.locator('#file-query').fill(''); return {opened:true};"
    )
    hold_real_reply(browser, "p02-alpha", root)
    browser.command(
        "await page.locator('#file-query').fill('p02 beta'); await page.locator('#file-query').fill('p02-alpha'); "
        "await page.waitForTimeout(200); "
        "if (await page.locator('#file-results button').count()) throw new Error('Obsolete results visible while replacement pending'); return {queryFence:true};"
    )
    release_reply(browser)
    wait_result(browser, fixture["keyboard"])
    browser.command(
        "const queries = p02Gate.searches.filter(p => p.query === 'p02-alpha'); "
        "if (queries.length !== 2 || queries[0].cancellationToken === queries[1].cancellationToken) throw new Error('Replacement search identities were not unique'); "
        "await page.unroute('**/rpc'); return {uniqueQueries:true};"
    )
    # A real old-root reply must not be used after the user changes the search root.
    hold_real_reply(browser, "p02-root", root)
    browser.command(
        f"await page.locator('#cwd').fill({json.dumps(fixture['alternate_root'])}); "
        "await page.waitForTimeout(200); if (await page.locator('#file-results button').count()) throw new Error('Old root results visible'); return {rootFence:true};"
    )
    release_reply(browser)
    wait_result(browser, fixture["alternate"])
    expected += (
        "@"
        + json.dumps(
            str(Path(fixture["alternate_root"]) / fixture["alternate"]),
            ensure_ascii=False,
        )
        + " "
    )
    browser.command(
        f"if ((await page.locator('#file-results').innerText()).includes({json.dumps(fixture['original'])})) throw new Error('Earlier root result survived'); "
        "await page.unroute('**/rpc'); "
        f"await page.locator('#file-results button').filter({{hasText:{json.dumps(fixture['alternate'])}}}).click(); "
        f"if (await page.locator('#prompt').inputValue() !== {json.dumps(expected)}) throw new Error('Reference did not retain selected root'); "
        f"await page.locator('#cwd').fill({json.dumps(root)}); return {{newRootInsertion:true}};"
    )
    browser.command(
        "await page.locator('#find-file').click(); await page.locator('#file-query').fill(''); return {cancelSearch:true};"
    )
    hold_real_reply(browser, "p02-alpha", root)
    browser.command(
        "await page.locator('#file-query').press('Escape'); p02Gate.release(); await page.waitForTimeout(200); if (await page.locator('#file-picker').isVisible() || await page.locator('#file-results button').count()) throw new Error('Closed search revived'); await page.unroute('**/rpc'); return {closeFence:true};"
    )
    browser.command(
        "await page.locator('#find-file').click(); await page.locator('#file-query').fill(''); return {taskSwitchSearch:true};"
    )
    hold_real_reply(browser, "p02-alpha", root)
    browser.command(
        "await page.locator('#new-task').click(); p02Gate.release(); await page.waitForTimeout(200); "
        "if (await page.locator('#file-picker').isVisible() || await page.locator('#file-results button').count()) throw new Error('Old task search revived'); "
        f"if (await page.locator('#prompt').inputValue() !== {json.dumps(expected)}) throw new Error('Task switch destroyed draft'); "
        "await page.unroute('**/rpc'); "
        f"await page.locator({json.dumps('.thread[data-thread-id=' + json.dumps(thread_id) + ']')}).click(); "
        f"await page.waitForFunction(root => document.querySelector('#cwd').value === root && document.querySelector('#messages').textContent.includes('Preserve this real legacy conversation through manual migration.'), {json.dumps(root)}); return {{taskFence:true}};"
    )
    screenshot(browser, work / f"search-inserted-{cycle}.png")
    return expected.strip(), {
        "passed": True,
        "backend": "real frozen-host native fuzzyFileSearch; no selected search component",
        "verified": [
            "real filesystem results",
            "relative root rejected without RPC or draft/input changes",
            "click insertion with spaces/quotes/backslash",
            "keyboard insertion without submitting",
            "draft preservation",
            "A-B-A query fence",
            "root replacement fence",
            "explicit close fence",
            "task replacement fence",
        ],
        "response_barriers": "Delayed actual route.fetch responses; result bodies unchanged",
    }
