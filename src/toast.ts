import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

type Entry = { id: number; from: string; to: string; rule: string; folder: string };
const root = document.getElementById("toast")!;
let timer: number | undefined;
const LIFE = 6000;

function base(p: string) { return p.split(/[\\/]/).pop() ?? p; }

function render(entries: Entry[]) {
  const e = entries[entries.length - 1];
  const more = entries.length > 1 ? ` and ${entries.length - 1} more` : "";
  root.innerHTML = `
    <div class="toast">
      <div class="icon">✓</div>
      <div class="text">Moved <b>${esc(base(e.from))}</b>${more} to <b>${esc(e.folder)}</b></div>
      <button class="pill" id="undo">Put back</button>
      <svg class="ring" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9" fill="none" stroke="var(--line)" stroke-width="3"/><circle id="arc" cx="12" cy="12" r="9" fill="none" stroke="var(--accent)" stroke-width="3" stroke-dasharray="56.5" stroke-dashoffset="0" transform="rotate(-90 12 12)"/></svg>
    </div>`;
  const arc = document.getElementById("arc") as unknown as SVGCircleElement;
  const t0 = performance.now();
  const tick = () => {
    const k = Math.min(1, (performance.now() - t0) / LIFE);
    arc.setAttribute("stroke-dashoffset", String(56.5 * k));
    if (k < 1 && timer !== undefined) requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
  document.getElementById("undo")!.onclick = async () => {
    for (const x of entries) { try { await invoke("undo", { id: x.id }); } catch {} }
    hide();
  };
  const box = root.firstElementChild as HTMLElement;
  box.onmouseenter = () => { if (timer) clearTimeout(timer); timer = undefined; };
  box.onmouseleave = () => arm(2500);
}

function esc(s: string) { return s.replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]!)); }
function arm(ms: number) { if (timer) clearTimeout(timer); timer = window.setTimeout(hide, ms); }
function hide() { if (timer) clearTimeout(timer); timer = undefined; root.innerHTML = ""; invoke("hide_toast"); }

const system = window.matchMedia("(prefers-color-scheme: dark)");
let setting = "system";
const applyTheme = () => document.documentElement.dataset.theme = setting === "dark" || (setting === "system" && system.matches) ? "dark" : "light";
applyTheme(); system.addEventListener("change", applyTheme);

listen<{ entries: Entry[]; theme: string }>("toast", (ev) => { setting = ev.payload.theme ?? "system"; applyTheme(); render(ev.payload.entries); arm(LIFE); });
