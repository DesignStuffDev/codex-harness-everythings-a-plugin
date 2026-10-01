"use strict";
// DOM presentation is separate from request ownership and stale-result fencing.
function createFilePicker({rpc, context, byId}) {
  const panel = byId("file-picker"), query = byId("file-query"), list = byId("file-results"), prompt = byId("prompt");
  let current = {open: false, files: []}, highlighted = 0, selection = null;
  function highlight(index) {
    highlighted = Math.max(0, Math.min(index, current.files.length - 1));
    [...list.children].forEach((node, position) => node.setAttribute("aria-selected", String(position === highlighted)));
    const active = list.children[highlighted];
    if (active) { query.setAttribute("aria-activedescendant", active.id); active.scrollIntoView({block: "nearest"}); }
    else query.removeAttribute("aria-activedescendant");
  }
  function render(state) {
    current = state; panel.hidden = !state.open;
    byId("find-file").setAttribute("aria-expanded", String(state.open));
    byId("file-root").textContent = state.root || "No working directory";
    byId("file-root").title = state.root || "";
    byId("file-search-error").textContent = state.error; byId("file-search-error").hidden = !state.error;
    byId("file-search-status").textContent = state.status === "loading" ? "Searching…" : state.status === "idle" ? "Type a file name" : state.status === "ready" ? state.files.length ? `${state.files.length} results · Enter to insert a reference` : "No matching files" : "";
    list.replaceChildren();
    state.files.forEach((file, index) => {
      const button = document.createElement("button"); button.type = "button"; button.role = "option"; button.id = `file-result-${index}`; button.tabIndex = -1;
      const path = document.createElement("span"); path.textContent = file.path;
      const kind = document.createElement("small"); kind.textContent = file.match_type === "directory" ? "Folder" : "File";
      button.append(path, kind); button.onclick = () => choose(index, state); list.append(button);
    });
    highlight(0);
  }
  const controller = new CodexFileSearch.SearchController({rpc, changed: render});
  function close(restoreFocus = false) { controller.close(); if (restoreFocus) prompt.focus(); }
  function search() { controller.search(query.value, context()); }
  function open() {
    selection = {text: prompt.value, start: prompt.selectionStart, end: prompt.selectionEnd};
    search(); query.focus(); query.select();
  }
  function choose(index, rendered = current) {
    const file = current.files[index], now = context();
    if (rendered !== current || !current.open || !file || now.root !== current.root || controller.context !== JSON.stringify([now.root, now.threadId, now.connection, now.pathStyle || "posix"])) { close(); return; }
    const range = selection?.text === prompt.value ? selection : {start: prompt.selectionStart, end: prompt.selectionEnd};
    const inserted = CodexFileSearch.insertReference(prompt.value, range.start, range.end, file);
    prompt.value = inserted.text; close(); prompt.focus(); prompt.setSelectionRange(inserted.caret, inserted.caret);
    prompt.dispatchEvent(new Event("input", {bubbles: true}));
  }
  byId("find-file").onclick = () => current.open ? close(true) : open();
  byId("close-file-picker").onclick = () => close(true);
  byId("retry-file-search").onclick = search;
  query.oninput = search;
  query.onkeydown = (event) => {
    if (event.isComposing) return;
    if (event.key === "Escape") { event.preventDefault(); close(true); }
    else if (event.key === "ArrowDown" || event.key === "ArrowUp") { event.preventDefault(); highlight(highlighted + (event.key === "ArrowDown" ? 1 : -1)); }
    else if (event.key === "Enter") { event.preventDefault(); if (current.files.length) choose(highlighted); }
  };
  return {close, rootChanged() { if (current.open) search(); }};
}
