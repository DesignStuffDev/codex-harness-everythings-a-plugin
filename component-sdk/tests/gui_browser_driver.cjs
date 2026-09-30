// Persistent manual Chromium driver. The requested in-app Browser is separate;
// using this driver must be reported as the Playwright fallback.
const fs = require('node:fs');
const readline = require('node:readline');
const { chromium } = require(process.env.CODEX_PLAYWRIGHT_MODULE || 'playwright');

const options = new Map();
for (let i = 2; i < process.argv.length; i += 2) {
  options.set(process.argv[i], process.argv[i + 1]);
}
const readyFile = options.get('--ready-file');
const reportFile = options.get('--report-file');
if (!readyFile || !reportFile) {
  throw new Error('Provide --ready-file and --report-file');
}
const report = { browser: 'Chromium through Playwright; not in-app Browser', commands: [], pageErrors: [], closed: false };
const save = () => fs.writeFileSync(reportFile, JSON.stringify(report, null, 2) + '\n');

(async () => {
  const browser = await chromium.launch({
    executablePath: process.env.CODEX_BROWSER_BIN || '/usr/bin/chromium',
    args: ['--no-sandbox'],
  });
  const page = await browser.newPage({ viewport: { width: 1440, height: 960 } });
  page.on('pageerror', error => {
    report.pageErrors.push(error.message);
    save();
    console.log('PAGEERROR ' + error.message);
  });
  // Read the private bearer URL internally; never log it or place it in commands.
  async function gotoReadyFile(path) {
    const ready = fs.readFileSync(path, 'utf8').split('\n').filter(Boolean)
      .map(line => JSON.parse(line))
      .find(event => event.method === 'presentation/ready');
    if (!ready) throw new Error('Presentation is not ready');
    await page.goto(ready.params.url);
    await page.locator('#prompt').waitFor();
    return { title: await page.title() };
  }
  report.initial = await gotoReadyFile(readyFile);
  save();
  console.log('READY ' + report.initial.title);
  const input = readline.createInterface({ input: process.stdin, output: process.stdout });
  try {
    for await (const command of input) {
      if (command === 'close') break;
      const entry = { command, started: new Date().toISOString() };
      report.commands.push(entry);
      try {
        entry.result = await eval('(async()=>{' + command + '})()');
        entry.passed = true;
        console.log(JSON.stringify(entry.result ?? null));
      } catch (error) {
        entry.passed = false;
        entry.error = error.message;
        console.log('ERROR ' + error.message);
      }
      save();
    }
  } finally {
    await browser.close();
    input.close();
    report.closed = true;
    save();
  }
})().catch(error => {
  // Exceptions may contain a private navigation URL. Keep diagnostics local.
  report.driverError = error.message;
  save();
  console.error('Browser driver failed; inspect the private report file.');
  process.exitCode = 1;
});
