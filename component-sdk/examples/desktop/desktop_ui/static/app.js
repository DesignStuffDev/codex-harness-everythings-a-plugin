"use strict";
const $ = (id) => document.getElementById(id);
const tokenFromUrl = new URLSearchParams(location.hash.slice(1)).get("token");
if (tokenFromUrl) { sessionStorage.setItem("desktopToken", tokenFromUrl); history.replaceState(null, "", "/"); }
const token = sessionStorage.getItem("desktopToken") || "";
let selected = null, activeTurn = null, events, threadCursor, turnCursor, selecting = 0, submitting = false;
const items = new Map(), requests = new Map(), completedTurns = new Map();
function error(message) { $("error").textContent = message; $("error").hidden = !message; }
function connection(text, connected) { $("connection").classList.toggle("connected", connected); $("connection").lastElementChild.textContent = text; }
function element(tag, text, className) { const node = document.createElement(tag); node.textContent = text; if (className) node.className = className; return node; }
async function call(path, body) {
  const response = await fetch(path, {method: body ? "POST" : "GET", headers: {"Authorization": `Bearer ${token}`, "Content-Type": "application/json"}, body: body ? JSON.stringify(body) : undefined});
  const value = await response.json();
  if (!response.ok || value.error) throw new Error(value.error?.message || value.error || "Codex request failed");
  return value;
}
async function rpc(method, params) { return (await call("/rpc", {method, params})).result; }
function safely(action) { return (...args) => Promise.resolve(action(...args)).catch((reason) => error(reason.message)); }
function running(turn, status = null) {
  if (completedTurns.has(turn)) { status = completedTurns.get(turn); turn = null; }
  activeTurn = turn; $("stop").hidden = !turn; $("send").disabled = Boolean(turn) || submitting;
  $("run-status").textContent = turn ? "Working on your task" : ({interrupted: "Task interrupted", failed: "Task failed", completed: "Task complete"}[status] || "Ready when you are");
  $("activity").textContent = turn ? "Codex is working…" : status === "interrupted" ? "Stopped. You can send another message." : status === "failed" ? "The task failed. Review the error before retrying." : "";
}
function settleTools(turnId, status) {
  for (const node of items.values()) {
    if (node.dataset.turnId === turnId && node.dataset.pending === "true" && !["userMessage", "agentMessage"].includes(node.dataset.type)) {
      const outcome = status === "interrupted" ? "interrupted" : status === "failed" ? "stopped after turn failure" : "ended · final tool status unavailable";
      node.firstChild.textContent = `${node.dataset.type.replace(/([A-Z])/g, " $1")} · ${outcome}`; node.dataset.pending = "false";
    }
  }
}
function renderItem(item, prepend = false, turnId = null) {
  if (!item?.id) return;
  let node = items.get(item.id);
  if (!node) { node = element("article", "", "message"); node.append(element("div", "", "message-label"), element("pre", "")); items.set(item.id, node); if (prepend) $("messages").prepend(node); else $("messages").append(node); }
  node.dataset.type = item.type;
  if (turnId) node.dataset.turnId = turnId;
  node.dataset.pending = String(item.status === "inProgress");
  const user = item.type === "userMessage", assistant = item.type === "agentMessage";
  node.className = `message ${user ? "user" : assistant ? "assistant" : "tool"}`;
  node.firstChild.textContent = user ? "YOU" : assistant ? "CODEX" : `${item.type.replace(/([A-Z])/g, " $1")} · ${item.status || ""}`;
  const text = user ? item.content.map((part) => part.text || `[${part.type}]`).join("\n") : assistant || item.type === "plan" ? item.text : item.type === "commandExecution" ? `${item.command}\n${item.aggregatedOutput || ""}` : item.type === "reasoning" ? (item.summary || []).join("\n") : JSON.stringify(item, null, 2);
  node.lastChild.textContent = text || "";
  if (completedTurns.has(node.dataset.turnId)) settleTools(node.dataset.turnId, completedTurns.get(node.dataset.turnId));
}
function scrollMessages() { const box = $("conversation"); if (box.scrollHeight - box.scrollTop - box.clientHeight < 350) box.scrollTop = box.scrollHeight; }
async function listThreads(more = false) {
  const page = await rpc("thread/list", {limit: 30, sortKey: "updated_at", ...(more && threadCursor ? {cursor: threadCursor} : {})});
  if (!more) $("threads").replaceChildren();
  for (const thread of page.data) {
    const button = element("button", thread.name || thread.preview || "Untitled task", "thread");
    button.title = thread.name || thread.preview || thread.id; button.dataset.threadId = thread.id;
    button.append(element("small", new Date(thread.updatedAt * 1000).toLocaleDateString(undefined, {month: "short", day: "numeric"})));
    button.classList.toggle("selected", thread.id === selected); button.onclick = safely(() => selectThread(thread.id)); $("threads").append(button);
  }
  threadCursor = page.nextCursor; $("more-threads").hidden = !threadCursor;
}
async function loadTurns(older = false, generation = selecting) {
  const threadId = selected;
  const page = await rpc("thread/turns/list", {threadId, limit: 20, sortDirection: "desc", itemsView: "full", ...(older && turnCursor ? {cursor: turnCursor} : {})});
  if (generation !== selecting) return;
  const collected = []; let runningTurn = null;
  for (const turn of [...page.data].reverse()) {
    let turnItems = turn.items;
    if (turn.itemsView !== "full") {
      turnItems = []; let cursor = null;
      do { const itemPage = await rpc("thread/items/list", {threadId, turnId: turn.id, limit: 100, ...(cursor ? {cursor} : {})}); if (generation !== selecting) return; turnItems.push(...itemPage.data.map((entry) => entry.item)); cursor = itemPage.nextCursor; } while (cursor);
    }
    collected.push(...turnItems.map((item) => ({item, turnId: turn.id})));
    if (turn.status === "inProgress") runningTurn = turn.id;
    else completedTurns.set(turn.id, turn.status);
  }
  if (generation !== selecting) return;
  if (runningTurn) running(runningTurn); else if (!older) running(null, page.data[0]?.status);
  if (older) collected.reverse().forEach(({item, turnId}) => renderItem(item, true, turnId)); else collected.forEach(({item, turnId}) => renderItem(item, false, turnId));
  turnCursor = page.nextCursor; $("older").hidden = !turnCursor;
}
async function selectThread(id) {
  if (submitting) return;
  const generation = ++selecting; selected = id; turnCursor = null; items.clear(); $("messages").replaceChildren(); running(null); error("");
  $("welcome").hidden = true; $("conversation").hidden = false; $("sidebar").classList.remove("open"); localStorage.setItem("codexSelectedThread", id);
  const response = await rpc("thread/resume", {threadId: id, excludeTurns: true});
  if (generation !== selecting) return;
  $("task-title").textContent = response.thread.name || response.thread.preview || "New task"; $("cwd").value = response.thread.cwd || "";
  document.querySelectorAll(".thread").forEach((button) => button.classList.toggle("selected", button.dataset.threadId === id));
  await loadTurns(false, generation); if (generation === selecting) $("conversation").scrollTop = $("conversation").scrollHeight;
}
function newTask() {
  if (submitting) return;
  ++selecting; selected = null; activeTurn = null; localStorage.removeItem("codexSelectedThread"); items.clear(); $("messages").replaceChildren();
  $("welcome").hidden = false; $("conversation").hidden = true; $("task-title").textContent = "New task"; $("sidebar").classList.remove("open");
  document.querySelectorAll(".thread").forEach((button) => button.classList.remove("selected")); running(null); error(""); $("prompt").focus();
}
async function submit(event) {
  event.preventDefault(); const text = $("prompt").value.trim(); if (!text || submitting || activeTurn) return;
  submitting = true; $("send").disabled = true; error("");
  try {
    if (!selected) { const params = {ephemeral: false}; if ($("cwd").value.trim()) params.cwd = $("cwd").value.trim(); const created = await rpc("thread/start", params); selected = created.thread.id; ++selecting; localStorage.setItem("codexSelectedThread", selected); $("welcome").hidden = true; $("conversation").hidden = false; }
    const result = await rpc("turn/start", {threadId: selected, input: [{type: "text", text, text_elements: []}]});
    if ($("prompt").value.trim() === text) $("prompt").value = ""; running(result.turn.status === "inProgress" ? result.turn.id : null, result.turn.status); $("task-title").textContent = text.slice(0, 90); await listThreads();
  } finally { submitting = false; $("send").disabled = Boolean(activeTurn); }
}
async function reply(request, result, failure) {
  await call("/reply", {id: request.id, ...(failure ? {error: {code: -32000, message: "Unsupported interaction canceled by user"}} : {result})});
  requests.delete(request.id); renderRequests();
}
function renderRequests() {
  $("approvals").replaceChildren();
  for (const request of requests.values()) {
    const params = request.params || {}, card = element("article", "", "approval");
    card.append(element("h3", "Codex needs your decision"));
    if (params.threadId && params.threadId !== selected) { const link = element("button", "Open related task"); link.onclick = safely(() => selectThread(params.threadId)); card.append(link); }
    card.append(element("pre", params.reason || params.command || params.cwd || request.method));
    const action = (label, result, failure = false) => { const button = element("button", label); button.onclick = safely(async () => { button.disabled = true; try { await reply(request, result, failure); } finally { button.disabled = false; } }); card.append(button); };
    if (["item/commandExecution/requestApproval", "item/fileChange/requestApproval"].includes(request.method)) {
      if (params.command && params.reason) card.append(element("pre", params.command));
      for (const field of ["kind", "additionalPermissions", "networkApprovalContext", "grantRoot"]) if (params[field]) card.append(element("pre", typeof params[field] === "string" ? params[field] : JSON.stringify(params[field], null, 2)));
      action("Allow once", {decision: "accept"}); action("Decline", {decision: "decline"}); action("Cancel turn", {decision: "cancel"});
    } else if (request.method === "item/permissions/requestApproval") {
      card.append(element("pre", JSON.stringify(params.permissions, null, 2))); action("Allow for this turn", {permissions: params.permissions, scope: "turn"}); action("Decline", {permissions: {}, scope: "turn"});
    } else if (request.method === "item/tool/requestUserInput") {
      const controls = [];
      for (const question of params.questions) {
        const label = element("label", question.question), input = document.createElement("input"); input.type = question.isSecret ? "password" : "text"; input.autocomplete = "off";
        if (question.options?.length) { const select = document.createElement("select"); for (const option of question.options) select.append(new Option(option.label, option.label)); if (question.isOther) select.append(new Option("Write an answer", "")); select.onchange = () => { input.hidden = Boolean(select.value); input.value = select.value; }; input.value = select.value; input.hidden = true; label.append(select); }
        label.append(input); card.append(label); controls.push([question.id, input]);
      }
      const button = element("button", "Submit answers"); button.onclick = safely(() => reply(request, {answers: Object.fromEntries(controls.map(([id, input]) => [id, {answers: [input.value]}]))})); card.append(button);
    } else { card.append(element("p", "This interaction is not supported by this desktop version.")); action("Cancel request", null, true); }
    $("approvals").append(card);
  }
}
function notification(message) {
  const {method, params = {}} = message;
  if ("id" in message) { requests.set(message.id, message); renderRequests(); return; }
  if (method === "serverRequest/resolved") { requests.delete(params.requestId); renderRequests(); return; }
  if (method === "error") { error(params.error?.message || "Codex reported an error"); return; }
  if (params.threadId !== selected) return;
  if (method === "turn/started") running(params.turn.id);
  if (method === "turn/completed") { completedTurns.set(params.turn.id, params.turn.status); if (completedTurns.size > 512) completedTurns.delete(completedTurns.keys().next().value); settleTools(params.turn.id, params.turn.status); running(null, params.turn.status); if (params.turn.error) error(params.turn.error.message); safely(listThreads)(); }
  if (method === "item/started" || method === "item/completed") renderItem(params.item, false, params.turnId || activeTurn);
  if (method === "item/agentMessage/delta" || method === "item/commandExecution/outputDelta") {
    if (!items.has(params.itemId)) renderItem({id: params.itemId, type: method.includes("agentMessage") ? "agentMessage" : "commandExecution", text: "", command: "", aggregatedOutput: "", status: "inProgress"}, false, params.turnId || activeTurn);
    items.get(params.itemId).lastChild.textContent += params.delta;
  }
  if (method === "item/mcpToolCall/progress") $("activity").textContent = params.message || "A tool is working…";
  scrollMessages();
}
async function connect() {
  events?.close(); const status = await call("/status"); if (!status.connected) throw new Error("Codex has stopped. Relaunch the desktop component to reconnect.");
  requests.clear(); status.requests.forEach((request) => requests.set(request.id, request)); renderRequests();
  events = new EventSource(`/events?token=${encodeURIComponent(token)}&after=${status.cursor}`);
  events.onopen = () => connection("Connected", true); events.onerror = () => connection("Reconnecting", false);
  events.onmessage = (event) => notification(JSON.parse(event.data));
  events.addEventListener("disconnected", () => { events.close(); connection("Disconnected", false); error("Codex has stopped. Relaunch the desktop component; your saved tasks remain available."); });
  events.addEventListener("reset", safely(async () => { await connect(); if (selected) await selectThread(selected); }));
}
$("composer").onsubmit = safely(submit); $("new-task").onclick = newTask; $("menu").onclick = () => $("sidebar").classList.toggle("open");
$("refresh").onclick = safely(() => listThreads()); $("more-threads").onclick = safely(() => listThreads(true)); $("older").onclick = safely(() => loadTurns(true));
$("stop").onclick = safely(async () => { if (selected && activeTurn) await rpc("turn/interrupt", {threadId: selected, turnId: activeTurn}); });
$("prompt").onkeydown = (event) => { if (event.key === "Enter" && !event.shiftKey && !event.isComposing) { event.preventDefault(); $("composer").requestSubmit(); } };
document.addEventListener("keydown", (event) => { if ((event.metaKey || event.ctrlKey) && event.key === "k") { event.preventDefault(); newTask(); } });
document.querySelectorAll("[data-prompt]").forEach((button) => { button.onclick = () => { $("prompt").value = button.dataset.prompt; $("prompt").focus(); }; });
safely(async () => { await connect(); await listThreads(); const saved = localStorage.getItem("codexSelectedThread"); if (saved) await selectThread(saved); })();
