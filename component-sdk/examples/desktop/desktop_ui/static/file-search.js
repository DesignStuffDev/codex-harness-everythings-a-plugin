"use strict";
// Existing App Server one-shot RPCs; generation fencing is independent of best-effort cancellation.
(function (scope) {
  class SearchController {
    constructor({rpc, changed, pageId = crypto.randomUUID(), delay = 150, schedule = (callback, milliseconds) => setTimeout(callback, milliseconds), unschedule = (timer) => clearTimeout(timer)}) {
      Object.assign(this, {rpc, changed, pageId, delay, schedule, unschedule});
      this.revision = 0; this.timer = null; this.active = null; this.queued = null;
      this.state = {open: false, query: "", root: "", status: "idle", files: [], error: ""};
    }
    publish() { this.changed({...this.state, files: [...this.state.files]}); }
    current(request) { return this.state.open && request.revision === this.revision && request.context === this.context; }
    cancelActive() {
      if (!this.active || this.active.cancelled) return;
      this.active.cancelled = true;
      // A token belongs to exactly one query. A delayed cancel cannot cancel its successor.
      Promise.resolve(this.rpc("fuzzyFileSearch", {query: "", roots: [this.active.root], cancellationToken: this.active.token})).catch(() => {});
    }
    search(query, {root, threadId, connection, pathStyle = "posix"}) {
      this.revision += 1;
      if (!Number.isSafeInteger(this.revision)) throw new Error("Reopen the desktop to continue searching.");
      this.context = JSON.stringify([root, threadId, connection, pathStyle]);
      this.unschedule(this.timer); this.timer = null; this.queued = null; this.cancelActive();
      this.state = {open: true, query, root, status: query.trim() ? "loading" : "idle", files: [], error: ""};
      if (!root) { this.state.status = "error"; this.state.error = "Choose a working directory to search for files."; }
      else if (!absoluteRoot(root, pathStyle)) { this.state.status = "error"; this.state.error = "File search needs an absolute working directory. Enter the full directory path."; }
      this.publish();
      if (!query.trim() || this.state.status === "error") return;
      const request = {revision: this.revision, context: this.context, query, root, token: `${this.pageId}:${this.revision}`, cancelled: false};
      this.timer = this.schedule(() => { this.timer = null; this.run(request); }, this.delay);
    }
    async run(request) {
      if (!this.current(request)) return;
      // Retain only one active request and the latest successor, even on a slow filesystem.
      if (this.active) { this.queued = request; return; }
      this.active = request;
      try {
        const result = await this.rpc("fuzzyFileSearch", {query: request.query, roots: [request.root], cancellationToken: request.token});
        if (!this.current(request)) return;
        if (!Array.isArray(result?.files) || result.files.some((file) => !validFile(file, request.root))) throw new Error("Codex returned an invalid file search result.");
        this.state.files = result.files; this.state.status = "ready"; this.state.error = ""; this.publish();
      } catch (error) {
        if (this.current(request)) { this.state.files = []; this.state.status = "error"; this.state.error = error.message || "File search failed. Try again."; this.publish(); }
      } finally {
        this.active = null;
        const next = this.queued; this.queued = null;
        if (next) this.run(next);
      }
    }
    close() {
      this.revision += 1; this.unschedule(this.timer); this.timer = null; this.queued = null; this.cancelActive();
      this.state = {...this.state, open: false, files: [], error: "", status: "idle"}; this.publish();
    }
  }
  function absoluteRoot(root, pathStyle = "posix") {
    if (pathStyle === "posix") return root.startsWith("/");
    if (pathStyle === "windows") return /^[a-z]:[\\/]/iu.test(root) || /^[\\/]{2}[^\\/]+[\\/][^\\/]+(?:[\\/]|$)/u.test(root);
    return false;
  }
  function validFile(file, root) {
    return file && file.root === root && typeof file.path === "string" && file.path.length > 0
      && !/^[\\/]|^[a-z]:/iu.test(file.path) && !file.path.split(/[\\/]/u).includes("..")
      && ["file", "directory"].includes(file.match_type);
  }
  function fileReference(file) {
    const separator = file.root.includes("\\") && !file.root.includes("/") ? "\\" : "/";
    const fullPath = `${file.root.replace(/[\\/]+$/u, "")}${separator}${file.path}`;
    return `@${JSON.stringify(fullPath)}`;
  }
  function insertReference(text, start, end, file) {
    const before = text.slice(0, start), after = text.slice(end);
    const inserted = `${before && !/\s$/u.test(before) ? " " : ""}${fileReference(file)}${!after || !/^\s/u.test(after) ? " " : ""}`;
    return {text: before + inserted + after, caret: before.length + inserted.length};
  }
  const api = {SearchController, absoluteRoot, fileReference, insertReference};
  if (typeof module !== "undefined" && module.exports) module.exports = api;
  else scope.CodexFileSearch = api;
})(globalThis);
