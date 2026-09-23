import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open as pickFolder } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

// ---------- types (mirror config.rs / engine.rs) ----------
type Rule = { id: string; name: string; enabled: boolean; extensions: string[]; keywords: string[]; hosts: string[]; folder: string; wait_secs: number; builtin: boolean };
type Config = { setup_done: boolean; watched: string[]; dest_root: string; rules: Rule[]; cleanup_empty_days: number; autostart: boolean; notifications: boolean; sound: boolean; theme: string; paused: boolean; paused_until: number | null; ai_mode: string; only_new_since: number | null };
type Entry = { id: number; time: number; from: string; to: string; rule: string; folder: string; undone: boolean };
type Waiting = { name: string; folder: string; reason: string };
type Startup = { packaged: boolean; state: string; locked: boolean; note: string };
type Snapshot = { startup: Startup; config: Config; recent: Entry[]; filed_total: number; filed_today: number; waiting: Waiting[]; paused: boolean; version: string };
type FolderInfo = { name: string; path: string; files: number; exists: boolean; empty_since: number | null };
type Drive = { root: string; free: number; total: number };
type RuleHit = { id: string; count: number; examples: string[] };
type Suggestion = { id: string; name: string; folder: string; extensions: string[]; keywords: string[]; hosts: string[]; count: number; examples: string[]; kind: "type" | "site" | "word" | "split"; out_of: string };
type Scan = { total: number; hits: RuleHit[]; suggestions: Suggestion[]; leftovers: [string, number][] };

// ---------- state ----------
let snap: Snapshot;
let screen: "home" | "rules" | "folders" | "history" | "settings" = "home";
let setupStep = 0;
let draft: Config | null = null; // wizard copy
let existingChoice: "new" | "all" = "new";
let existingCount: number | null = null;
let scan: Scan | null = null;       // what the wizard found in the watched folders
let scanFor = "";                   // watched-folder list the scan belongs to
let accepted = new Set<string>();   // suggestion ids turned into rules
const app = document.getElementById("app")!;

// ---------- helpers ----------
function esc(s: string) { return s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]!)); }
function base(p: string) { return p.split(/[\\/]/).pop() ?? p; }
function dir(p: string) { const i = Math.max(p.lastIndexOf("\\"), p.lastIndexOf("/")); return i > 0 ? p.slice(0, i) : p; }
function ago(t: number) { const s = Math.max(0, (Date.now() - t) / 1000); if (s < 60) return "just now"; if (s < 3600) return `${Math.floor(s / 60)} min ago`; if (s < 86400) return `${Math.floor(s / 3600)} h ago`; return new Date(t).toLocaleDateString(); }
function gb(n: number) { return (n / 1e9).toFixed(0) + " GB"; }
function waitLabel(s: number) { return s === 0 ? "instantly" : s < 3600 ? `after ${s / 60} min` : `after ${s / 3600} h`; }
const WAITS = [0, 120, 300, 900, 3600];
function el<T extends HTMLElement>(sel: string, root: ParentNode = app) { return root.querySelector(sel) as T; }
function all<T extends HTMLElement>(sel: string, root: ParentNode = app) { return Array.from(root.querySelectorAll(sel)) as T[]; }

async function refresh() { snap = await invoke<Snapshot>("get_state"); applyTheme(); render(); }
async function save(cfg: Config) { await invoke("save_config", { config: cfg }); await refresh(); }
function applyTheme() {
  const t = snap?.config.theme ?? "system";
  const dark = t === "dark" || (t === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
}
window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", applyTheme);

// ---------- render ----------
function render() {
  if (!snap.config.setup_done) { renderSetup(); return; }
  const nav = (id: typeof screen, label: string) => `<button class="nav ${screen === id ? "active" : ""}" data-nav="${id}">${label}</button>`;
  app.innerHTML = `
    <div class="shell">
      <aside class="side">
        <div class="brand">FileZ</div>
        ${nav("home", "Home")}${nav("rules", "Rules")}${nav("folders", "Folders")}${nav("history", "History")}${nav("settings", "Settings")}
        <div class="spacer"></div>
        <div class="status">${snap.paused ? "Paused" : "Watching " + snap.config.watched.length + (snap.config.watched.length === 1 ? " folder" : " folders")}<br>v${snap.version}</div>
      </aside>
      <main class="main" id="main"></main>
    </div>`;
  all("[data-nav]").forEach((b) => (b.onclick = () => { screen = b.dataset.nav as typeof screen; refresh(); }));
  const main = el("#main");
  ({ home: renderHome, rules: renderRules, folders: renderFolders, history: renderHistory, settings: renderSettings })[screen](main);
}

// ---------- setup wizard ----------
async function renderSetup() {
  if (!draft) draft = structuredClone(snap.config);
  const d = draft;
  const dots = [0, 1, 2].map((i) => `<span class="dot ${i <= setupStep ? "on" : ""}"></span>`).join("");
  let body = "";
  if (setupStep === 0) {
    const known = await invoke<[string, string][]>("known_folders");
    const rows = known.map(([label, path]) => `<label class="item"><input type="checkbox" data-watch="${esc(path)}" ${d.watched.includes(path) ? "checked" : ""}> <div class="grow"><div class="name">${label}</div><div class="sub">${esc(path)}</div></div></label>`).join("");
    const extra = d.watched.filter((w) => !known.some(([, p]) => p === w)).map((w) => `<label class="item"><input type="checkbox" data-watch="${esc(w)}" checked> <div class="grow"><div class="name">${esc(base(w))}</div><div class="sub">${esc(w)}</div></div></label>`).join("");
    body = `<h1>Which folders should FileZ watch?</h1><p>New files landing in these folders get filed. Folders inside them are never touched.</p>
      <div class="card"><div class="list">${rows}${extra}</div><div class="row" style="margin-top:12px"><button class="pill" id="add-watch">Choose another folder</button></div></div>`;
  } else if (setupStep === 1) {
    const key = d.watched.join("|");
    if (!scan || scanFor !== key) {
      try { scan = await invoke<Scan>("scan_existing", { config: d }); } catch { scan = { total: 0, hits: [], suggestions: [], leftovers: [] }; }
      scanFor = key;
      // first look: turn on every shipped card that has files waiting for it
      for (const h of scan.hits) { const r = d.rules.find((x) => x.id === h.id); if (r && r.builtin && h.count > 0) r.enabled = true; }
    }
    const sc = scan;
    const hit = (id: string) => sc.hits.find((h) => h.id === id);
    const where = d.watched.length === 1 ? base(d.watched[0]) : "your folders";
    const shipped = d.rules.filter((r) => r.builtin).map((r) => { const h = hit(r.id); return `<button class="pill ${r.enabled ? "on" : ""} ${h ? "" : "dim"}" data-rule="${r.id}" title="${h ? esc(h.examples.join(", ")) : "nothing like this yet"}">${esc(r.name)}${h ? ` <b>${h.count}</b>` : ""}</button>`; }).join("");
    const sugg = sc.suggestions.map((g) => `<button class="pill sug ${accepted.has(g.id) ? "on" : ""}" data-sug="${g.id}" title="${esc(g.examples.join(", "))}">${esc(g.name)} <b>${g.count}</b><span class="ex">${g.out_of ? `out of ${esc(g.out_of)} \u00b7 ` : ""}${esc(g.examples.join(", "))}</span></button>`).join("");
    const left = sc.leftovers.slice(0, 12).map(([e, n]) => `<button class="pill ghost small" data-left="${esc(e)}" title="Make a category for .${esc(e)} files">.${esc(e)} · ${n}</button>`).join("");
    const custom = d.rules.filter((r) => !r.builtin && !r.id.startsWith("sug-")).map((r) => `<button class="pill ${r.enabled ? "on" : ""}" data-rule="${r.id}">${esc(r.name)}</button>`).join("");
    const tree = d.rules.filter((r) => r.enabled).map((r) => `  ├─ ${esc(r.folder)}`).join("\n");
    body = `<h1>Here’s what’s in ${esc(where)}</h1><p>${sc.total ? `${sc.total} files looked at. Bold numbers are files already there; greyed cards have nothing yet but still catch new downloads. Tap to turn any on or off.` : "Nothing there yet. These are the categories FileZ starts with; tap to turn any on or off."}</p>
      <div class="grid2"><div>
        <div class="card"><div class="pills">${shipped}${custom}</div></div>
        ${sugg ? `<div class="card"><h2>FileZ noticed these too</h2><p>Made from what is actually in the folder, including families hiding inside a broader category. Tap to give one its own folder; rename it later on the Rules page.</p><div class="pills">${sugg}</div></div>` : ""}
        ${left ? `<div class="card"><h2>Everything else</h2><p>Left where it is. Tap a type to give it a folder.</p><div class="pills">${left}</div></div>` : ""}
      </div>
      <div class="card"><h2>Your folders will look like</h2><div class="tree">${esc(base(d.dest_root))}\n${tree || "  (nothing yet)"}</div></div></div>`;
  } else {
    const drives = await invoke<Drive[]>("drives");
    const inside = d.watched[0] ?? d.dest_root;
    const bars = drives.map((x) => `<button class="pill ${d.dest_root.toUpperCase().startsWith(x.root) && d.dest_root !== inside ? "on" : ""}" data-drive="${x.root}">${x.root} · ${gb(x.free)} free of ${gb(x.total)}</button>`).join("");
    if (existingCount === null) { try { existingCount = await invoke<number>("count_existing", { config: d }); } catch { existingCount = 0; } }
    const existing = existingCount > 0 ? `<div class="card"><h2>${existingCount} ${existingCount === 1 ? "file is" : "files are"} already there and match a rule</h2>
      <div class="pills"><button class="pill ${existingChoice === "new" ? "on" : ""}" data-existing="new">Leave them, only tidy new files from now on</button><button class="pill ${existingChoice === "all" ? "on" : ""}" data-existing="all">Tidy them now as well</button></div>
      <p style="margin-top:8px">Either way, every move gets a Put back button.</p></div>` : "";
    body = `<h1>Where should the folders go?</h1><p>Same place is the simplest. Pick another drive if this one is small.</p>
      <div class="card"><div class="row"><button class="pill ${d.dest_root === inside ? "on" : ""}" id="dest-inside">Inside ${esc(base(inside))}</button><span class="muted">or on another drive:</span>${bars}<button class="pill" id="dest-pick">Choose folder</button></div>
      <p style="margin-top:12px">Folders will be created in <b>${esc(d.dest_root)}</b></p></div>${existing}`;
  }
  app.innerHTML = `<main class="main" style="max-width:860px;margin:0 auto"><div class="steps">${dots}</div>${body}
    <div class="row between" style="margin-top:8px"><button class="pill ghost" id="back" ${setupStep === 0 ? "disabled" : ""}>Back</button>
    <button class="pill primary" id="next">${setupStep === 2 ? "Start tidying" : "Next"}</button></div></main>`;
  all<HTMLInputElement>("[data-watch]").forEach((c) => (c.onchange = () => { const p = c.dataset.watch!; d.watched = c.checked ? [...new Set([...d.watched, p])] : d.watched.filter((w) => w !== p); }));
  el("#add-watch")?.addEventListener("click", async () => { const p = await pickFolder({ directory: true }); if (typeof p === "string") { d.watched = [...new Set([...d.watched, p])]; renderSetup(); } });
  all("[data-rule]").forEach((b) => (b.onclick = () => { const r = d.rules.find((x) => x.id === b.dataset.rule)!; r.enabled = !r.enabled; existingCount = null; renderSetup(); }));
  all("[data-sug]").forEach((b) => (b.onclick = () => {
    const g = scan!.suggestions.find((x) => x.id === b.dataset.sug)!;
    if (accepted.has(g.id)) { accepted.delete(g.id); d.rules = d.rules.filter((r) => r.id !== g.id); }
    else { accepted.add(g.id); d.rules.unshift({ id: g.id, name: g.folder, enabled: true, extensions: [...g.extensions], keywords: [...g.keywords], hosts: [...g.hosts], folder: g.folder, wait_secs: 120, builtin: false }); }
    existingCount = null; renderSetup();
  }));
  all("[data-left]").forEach((b) => (b.onclick = () => {
    const e = b.dataset.left!; if (e === "(no type)") return;
    const name = `${e.toUpperCase()} files`;
    d.rules.unshift({ id: "custom-" + e, name, enabled: true, extensions: [e], keywords: [], hosts: [], folder: name, wait_secs: 120, builtin: false });
    scan!.leftovers = scan!.leftovers.filter(([x]) => x !== e);
    existingCount = null; renderSetup();
  }));
  el("#dest-inside")?.addEventListener("click", () => { d.dest_root = d.watched[0] ?? d.dest_root; existingCount = null; renderSetup(); });
  all("[data-existing]").forEach((b) => (b.onclick = () => { existingChoice = b.dataset.existing as "new" | "all"; renderSetup(); }));
  all("[data-drive]").forEach((b) => (b.onclick = () => { d.dest_root = b.dataset.drive + "FileZ"; existingCount = null; renderSetup(); }));
  el("#dest-pick")?.addEventListener("click", async () => { const p = await pickFolder({ directory: true }); if (typeof p === "string") { d.dest_root = p; existingCount = null; renderSetup(); } });
  el("#back").onclick = () => { setupStep--; renderSetup(); };
  el("#next").onclick = async () => {
    if (setupStep === 0 && d.watched.length === 0) return;
    if (setupStep < 2) { if (setupStep === 0 && (d.dest_root === snap.config.dest_root)) d.dest_root = d.watched[0]; existingCount = null; setupStep++; renderSetup(); return; }
    d.only_new_since = existingChoice === "new" && (existingCount ?? 0) > 0 ? Date.now() : null;
    d.setup_done = true; draft = null; await save(d);
  };
}

// ---------- home ----------
function renderHome(main: HTMLElement) {
  const c = snap.config;
  const pauseText = snap.paused ? (c.paused_until ? `Paused until ${new Date(c.paused_until).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}` : "Paused until you turn it back on") : "FileZ is on";
  const recent = snap.recent.slice(0, 12);
  main.innerHTML = `
    <div class="grid2">
      <div class="card"><div class="row between"><div><h2>${pauseText}</h2><p>${snap.paused ? "Nothing moves while paused." : "Watching " + c.watched.map((w) => esc(base(w))).join(", ")}</p></div>
        <button class="toggle ${snap.paused ? "" : "on"}" id="pause-toggle" title="${snap.paused ? "Resume" : "Pause"}"></button></div>
        <div class="row" style="margin-top:8px">${snap.paused ? `<button class="pill primary" data-pause="0">Resume now</button>` : `<button class="pill" data-pause="60">Pause for 1 hour</button><button class="pill" data-pause="-1">Pause until I turn it back on</button>`}<button class="pill sage" id="tidy-now">Tidy now</button></div></div>
      <div class="card"><div class="row" style="gap:28px"><div><div class="big">${snap.filed_total}</div><div class="muted">files filed</div></div><div><div class="big">${snap.filed_today}</div><div class="muted">last 24 h</div></div><div><div class="big">${snap.waiting.length}</div><div class="muted">waiting</div></div></div>
        ${snap.waiting.length ? `<div class="list" style="margin-top:10px">${snap.waiting.slice(0, 5).map((w) => `<div class="item"><div class="grow"><div class="name">${esc(w.name)}</div><div class="sub">→ ${esc(w.folder)} · ${esc(w.reason)}</div></div></div>`).join("")}</div>` : ""}</div>
    </div>
    <div class="card"><h2>Recently filed</h2>
      ${recent.length ? `<div class="list">${recent.map(rowHtml).join("")}</div>` : `<div class="empty">Nothing filed yet. Download something and watch it land in the right folder.</div>`}</div>`;
  el("#pause-toggle").onclick = () => setPaused(!snap.paused, undefined);
  all("[data-pause]").forEach((b) => (b.onclick = () => { const m = Number(b.dataset.pause); setPaused(m !== 0, m > 0 ? m : undefined); }));
  el("#tidy-now").onclick = async () => { await invoke("tidy_now"); setTimeout(refresh, 1200); };
  wireRows(main);
}
async function setPaused(paused: boolean, minutes?: number) { await invoke("set_paused", { paused, minutes: minutes ?? null }); await refresh(); }

function rowHtml(e: Entry) {
  return `<div class="item ${e.undone ? "done" : ""}"><div class="grow"><div class="name">${esc(base(e.to))}</div><div class="sub">${esc(base(dir(e.from)))} → ${esc(e.folder)} · ${ago(e.time)}${e.undone ? " · put back" : ""}</div></div>
    <button class="pill ghost" data-reveal="${esc(e.undone ? e.from : e.to)}">Show</button>${e.undone ? "" : `<button class="pill" data-undo="${e.id}">Put back</button>`}</div>`;
}
function wireRows(root: HTMLElement) {
  all("[data-undo]", root).forEach((b) => (b.onclick = async () => { try { await invoke("undo", { id: Number(b.dataset.undo) }); } catch (err) { alert(String(err)); } refresh(); }));
  all("[data-reveal]", root).forEach((b) => (b.onclick = () => revealItemInDir(b.dataset.reveal!).catch(() => {})));
}

// ---------- rules ----------
type ListKey = "extensions" | "keywords" | "hosts";
const LIST_LABEL: Record<ListKey, string> = { extensions: "file types", keywords: "words in the name", hosts: "from websites" };
const LIST_HINT: Record<ListKey, string> = { extensions: "e.g. pdf", keywords: "e.g. invoice", hosts: "e.g. moodle" };
let editing: string | null = null; // "ruleId:listKey" of the chip list with an open input

function chipList(r: Rule, key: ListKey) {
  const chips = r[key].map((v, i) => `<span class="chip">${esc(v)}<b class="x" data-rm="${r.id}:${key}:${i}" title="Remove">&times;</b></span>`).join("");
  const input = editing === `${r.id}:${key}` ? `<input class="chip-input" data-in="${r.id}:${key}" placeholder="${LIST_HINT[key]}" autofocus>` : `<button class="pill ghost small" data-add="${r.id}:${key}">+ ${r[key].length ? "add" : LIST_LABEL[key]}</button>`;
  return `<span class="chips">${r[key].length ? `<span class="muted">${LIST_LABEL[key]}:</span>` : ""}${chips}${input}</span>`;
}

function renderRules(main: HTMLElement) {
  const c = snap.config;
  const from = c.watched.length === 1 ? base(c.watched[0]) : "any watched folder";
  const cards = c.rules.map((r) => {
    const waits = WAITS.map((w) => `<button class="pill ${r.wait_secs === w ? "on" : ""}" data-wait="${r.id}:${w}">${waitLabel(w)}</button>`).join("");
    return `<div class="card rule ${r.enabled ? "" : "off"}"><button class="toggle ${r.enabled ? "on" : ""}" data-toggle="${r.id}"></button>
      <div><div class="sentence"><b>${esc(r.name)}</b> from <span class="chip">${esc(from)}</span> go to <input class="folder" data-folder="${r.id}" value="${esc(r.folder)}"> <span class="pills">${waits}</span></div>
      <div class="lists">${chipList(r, "extensions")}${chipList(r, "keywords")}${chipList(r, "hosts")}</div>
      ${r.builtin ? "" : `<div class="row" style="margin-top:8px"><button class="pill ghost" data-del="${r.id}">Remove this rule</button></div>`}</div></div>`;
  }).join("");
  main.innerHTML = `<h1>Rules</h1><p>Top to bottom, first match wins. A file matches a card when its type, a word in its name, or the website it came from is listed. Files that match nothing stay where they are.</p>${cards}
    <div class="card"><div class="row"><b>Add your own rule</b><input class="folder" id="new-name" placeholder="Name, e.g. Invoices" style="width:220px"><button class="pill primary" id="new-rule">Add</button></div><p style="margin-top:8px">Then add file types, words or websites to it above. New rules go to a folder with the same name.</p></div>
    <div class="card"><div class="row between"><div><h2>Let FileZ suggest categories</h2><p>Looks at what is piling up unsorted in ${esc(from)} and offers a card for anything that repeats.</p></div><button class="pill" id="scan-now">Look now</button></div><div id="scan-out"></div></div>`;
  el("#scan-now").onclick = async () => {
    const out = el("#scan-out"); out.innerHTML = `<p class="muted">Looking…</p>`;
    let sc: Scan; try { sc = await invoke<Scan>("scan_existing", { config: c }); } catch (err) { out.innerHTML = `<p>${esc(String(err))}</p>`; return; }
    const sugg = sc.suggestions.map((g) => `<button class="pill sug" data-sug2="${g.id}" title="${esc(g.examples.join(", "))}">${esc(g.name)} <b>${g.count}</b><span class="ex">${g.out_of ? `out of ${esc(g.out_of)} \u00b7 ` : ""}${esc(g.examples.join(", "))}</span></button>`).join("");
    const left = sc.leftovers.filter(([e]) => e !== "(no type)").slice(0, 12).map(([e, n]) => `<button class="pill ghost small" data-left2="${esc(e)}">.${esc(e)} · ${n}</button>`).join("");
    out.innerHTML = sugg || left ? `${sugg ? `<div class="pills" style="margin-top:10px">${sugg}</div>` : ""}${left ? `<p style="margin-top:10px">Other unsorted types, tap to give one a folder:</p><div class="pills">${left}</div>` : ""}` : `<p style="margin-top:10px">Nothing unsorted is repeating right now. ${sc.total} files looked at.</p>`;
    all("[data-sug2]", out).forEach((b) => (b.onclick = () => { const g = sc.suggestions.find((x) => x.id === b.dataset.sug2)!; if (c.rules.some((r) => r.id === g.id)) return; c.rules.unshift({ id: g.id, name: g.folder, enabled: true, extensions: [...g.extensions], keywords: [...g.keywords], hosts: [...g.hosts], folder: g.folder, wait_secs: 120, builtin: false }); save(c); }));
    all("[data-left2]", out).forEach((b) => (b.onclick = () => { const e = b.dataset.left2!; const name = `${e.toUpperCase()} files`; c.rules.unshift({ id: "custom-" + e, name, enabled: true, extensions: [e], keywords: [], hosts: [], folder: name, wait_secs: 120, builtin: false }); save(c); }));
  };
  all("[data-toggle]").forEach((b) => (b.onclick = () => { const r = c.rules.find((x) => x.id === b.dataset.toggle)!; r.enabled = !r.enabled; save(c); }));
  all("[data-wait]").forEach((b) => (b.onclick = () => { const [id, w] = b.dataset.wait!.split(":"); c.rules.find((x) => x.id === id)!.wait_secs = Number(w); save(c); }));
  all<HTMLInputElement>("[data-folder]").forEach((i) => (i.onchange = () => { const r = c.rules.find((x) => x.id === i.dataset.folder)!; r.folder = i.value.trim() || r.name; save(c); }));
  all("[data-rm]").forEach((b) => (b.onclick = () => { const [id, key, idx] = b.dataset.rm!.split(":"); c.rules.find((x) => x.id === id)![key as ListKey].splice(Number(idx), 1); save(c); }));
  all("[data-add]").forEach((b) => (b.onclick = () => { editing = b.dataset.add!; render(); el<HTMLInputElement>("[data-in]")?.focus(); }));
  all<HTMLInputElement>("[data-in]").forEach((i) => {
    const commit = () => { const [id, key] = i.dataset.in!.split(":"); const v = i.value.trim().toLowerCase().replace(/^\./, ""); editing = null; if (v) { c.rules.find((x) => x.id === id)![key as ListKey].push(v); save(c); } else render(); };
    i.onkeydown = (e) => { if (e.key === "Enter") commit(); if (e.key === "Escape") { editing = null; render(); } };
    i.onblur = commit;
  });
  all("[data-del]").forEach((b) => (b.onclick = () => { c.rules = c.rules.filter((x) => x.id !== b.dataset.del); save(c); }));
  el("#new-rule").onclick = () => { const name = el<HTMLInputElement>("#new-name").value.trim(); if (!name) return; c.rules.unshift({ id: "custom-" + Date.now(), name, enabled: true, extensions: [], keywords: [], hosts: [], folder: name, wait_secs: 120, builtin: false }); save(c); };
}

// ---------- folders ----------
async function renderFolders(main: HTMLElement) {
  const c = snap.config;
  const tree = await invoke<FolderInfo[]>("folder_tree");
  const rows = tree.map((f) => `<div class="item"><div class="grow"><div class="name">${esc(f.name)} ${f.exists ? "" : '<span class="muted">(not created yet)</span>'}</div><div class="sub">${esc(f.path)}</div></div><span class="muted">${f.files} ${f.files === 1 ? "file" : "files"}</span>${f.exists ? `<button class="pill ghost" data-reveal="${esc(f.path)}">Open</button>` : ""}</div>`).join("");
  const days = [0, 7, 30].map((d) => `<button class="pill ${c.cleanup_empty_days === d ? "on" : ""}" data-days="${d}">${d === 0 ? "Never" : `after ${d} days`}</button>`).join("");
  main.innerHTML = `<h1>Folders</h1><p>Everything lands under <b>${esc(c.dest_root)}</b>.</p>
    <div class="card"><div class="list">${rows}</div></div>
    <div class="card"><h2>Remove category folders that stay empty</h2><div class="pills">${days}</div></div>`;
  all("[data-days]").forEach((b) => (b.onclick = () => { c.cleanup_empty_days = Number(b.dataset.days); save(c); }));
  all("[data-reveal]", main).forEach((b) => (b.onclick = () => revealItemInDir(b.dataset.reveal!).catch(() => {})));
}

// ---------- history ----------
function renderHistory(main: HTMLElement) {
  const rows = snap.recent.map(rowHtml).join("");
  main.innerHTML = `<h1>History</h1><div class="row between"><p>Newest first. "Put back" returns a file to where it came from and leaves it alone afterwards.</p><button class="pill" id="undo-today">Put everything from today back</button></div>
    <div class="card">${rows ? `<div class="list">${rows}</div>` : `<div class="empty">Nothing yet.</div>`}</div>`;
  el("#undo-today").onclick = async () => { if (!confirm("Put back every file Tidy moved in the last 24 hours?")) return; await invoke("undo_since", { since: Date.now() - 86_400_000 }); refresh(); };
  wireRows(main);
}

// ---------- settings ----------
function renderSettings(main: HTMLElement) {
  const c = snap.config;
  const tog = (key: "notifications", label: string, sub: string) => `<div class="item"><div class="grow"><div class="name">${label}</div><div class="sub">${sub}</div></div><button class="toggle ${c[key] ? "on" : ""}" data-set="${key}"></button></div>`;
  const themes = ["light", "dark", "system"].map((t) => `<button class="pill ${c.theme === t ? "on" : ""}" data-theme="${t}">${t[0].toUpperCase() + t.slice(1)}</button>`).join("");
  const watched = c.watched.map((w) => `<div class="item"><div class="grow"><div class="name">${esc(base(w))}</div><div class="sub">${esc(w)}</div></div><button class="pill ghost" data-unwatch="${esc(w)}" ${c.watched.length === 1 ? "disabled" : ""}>Stop watching</button></div>`).join("");
  const ai = [["rules", "Rules only (no AI)"], ["gemini", "Gemini API key"], ["local", "Your own local AI (Ollama)"], ["builtin", "Tiny built-in AI for naming files"]].map(([k, l]) => `<button class="pill ${c.ai_mode === k ? "on" : ""}" ${k === "rules" ? "" : "disabled"}>${l}</button>`).join("");
  const st = snap.startup;
  const startupOn = st.packaged ? st.state === "enabled" || st.state === "enabled-by-policy" : c.autostart;
  const startupRow = `<div class="item"><div class="grow"><div class="name">Start with Windows</div><div class="sub">${st.locked ? esc(st.note) : "FileZ sits in the tray and keeps watching."}</div></div><button class="toggle ${startupOn ? "on" : ""}" data-set="autostart" ${st.locked ? "disabled" : ""}></button></div>`;
  main.innerHTML = `<h1>Settings</h1>
    <div class="card"><div class="list">${startupRow}${tog("notifications", "Show a note when a file is filed", "Small toast at the bottom of the screen with a Put back button.")}</div></div>
    <div class="card"><h2>Look</h2><div class="pills">${themes}</div> <button class="pill ghost" id="preview-toast">Preview a toast</button></div>
    <div class="card"><h2>Watched folders</h2><div class="list">${watched}</div><div class="row" style="margin-top:10px"><button class="pill" id="add-watch">Watch another folder</button><span class="muted">Folders inside a watched folder are never touched.</span></div></div>
    <div class="card"><h2>Where files go</h2><div class="row"><span>${esc(c.dest_root)}</span><button class="pill" id="pick-dest">Change</button></div></div>
    <div class="card ai-locked"><h2>AI helpers <span class="chip">coming later</span></h2><p>Optional helpers for files the rules cannot place. Not in this version.</p><div class="pills">${ai}</div></div>`;
  all("[data-set]").forEach((b) => (b.onclick = () => { const k = b.dataset.set as "autostart" | "notifications"; c[k] = k === "autostart" ? !startupOn : !c[k]; save(c); }));
  all("[data-theme]").forEach((b) => (b.onclick = () => { c.theme = b.dataset.theme!; save(c); }));
  all("[data-unwatch]").forEach((b) => (b.onclick = () => { c.watched = c.watched.filter((w) => w !== b.dataset.unwatch); save(c); }));
  el("#add-watch").onclick = async () => { const p = await pickFolder({ directory: true }); if (typeof p === "string") { c.watched = [...new Set([...c.watched, p])]; save(c); } };
  el("#pick-dest").onclick = async () => { const p = await pickFolder({ directory: true, defaultPath: c.dest_root }); if (typeof p === "string") { c.dest_root = p; save(c); } };
  el("#preview-toast").onclick = () => invoke("preview_toast");
}

// ---------- boot ----------
function report(where: string, err: unknown) {
  const msg = `${where}: ${err instanceof Error ? err.stack || err.message : String(err)}`;
  invoke("log_frontend", { msg }).catch(() => {});
  app.innerHTML = `<div class="main"><div class="card"><h2>Tidy hit a problem</h2><pre style="white-space:pre-wrap;user-select:text">${esc(msg)}</pre></div></div>`;
}
window.addEventListener("error", (e) => report("window.error", e.error ?? e.message));
window.addEventListener("unhandledrejection", (e) => report("unhandled", e.reason));
listen("filed", () => refresh()).catch((e) => report("listen filed", e));
listen("config-changed", () => refresh()).catch((e) => report("listen config", e));
refresh().catch((e) => report("get_state", e));
// Home shows live countdowns; keep it current while it is on screen.
setInterval(() => { if (document.visibilityState === "visible" && snap?.config.setup_done && screen === "home") refresh().catch(() => {}); }, 5000);
