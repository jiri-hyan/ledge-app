// Ukázkový šuplík pro snímky a GIFy (kolo 24): `index.html?demo=1` mimo Tauri. Stejný App jako ve skutečnosti,
// jen host (`api.setDemoHost`) vrací smyšlená data ze `sets.ts` a události (`state`, `view`, `appearance`) posílá
// sběrnice `emitDemo`. V Tauri se nikdy nespustí (`index.tsx` ho načte jen mimo Tauri, `setDemoHost` tam nic nedělá).
// Parametry URL: set=studio|creator|minimal · style · theme · layout · accent · page · open=1 · ask=1 · nopending=1.
// Řízení z Playwrightu: `window.__demo` (open / close / page / style / ask / approve). Velikost, kterou by dostalo
// okno od Rustu (`report_size`), se pošle rodiči (`postMessage`) — scéna podle ní nastaví iframe.
import { emitDemo, setDemoHost } from "../api";
import { EMPTY_SNAPSHOT } from "../snapshot";
import { allSectionsOn } from "../cards/sections";
import { withStyle } from "../theme/styles";
import { normalizeAppearance } from "../theme/appearance";
import DEFAULTS from "../studio/demo-config.json";
import { costsFull, systemFull, weatherFull, type WxKind } from "./fixtures";
import { COVER, session, SET_IDS, askFor, buildSet, drawerWidth, type SetId } from "./sets";

const clone = <T,>(v: T): T => structuredClone(v);
export function installShowcase() {
  const q = new URLSearchParams(location.search);
  const now = Date.now();
  const setId: SetId = (SET_IDS as readonly string[]).includes(q.get("set") ?? "") ? (q.get("set") as SetId) : "studio";
  const data = buildSet(setId, now);

  const cfg: any = clone(DEFAULTS);
  cfg.onboarding = { ...(cfg.onboarding ?? {}), done: true };
  cfg.autostart = false;
  const style = q.get("style");
  let a: any = style ? withStyle(cfg.appearance, style) : normalizeAppearance({ ...cfg.appearance, ...data.appearance });
  if (q.get("theme")) a.theme = q.get("theme");
  if (q.get("layout")) a.layout = q.get("layout");
  if (q.get("material")) a.material = q.get("material");
  if (q.get("accent")) a.accent = "#" + q.get("accent");
  if (q.get("width")) a.width = q.get("width");
  if (q.get("w")) a.size = { ...(a.size ?? {}), width: Number(q.get("w")) };
  cfg.appearance = a;
  cfg.layout = clone(data.layout);
  // `cards=claude,notes` nechá na první záložce jen vyjmenované karty (soustředěné snímky)
  if (q.get("cards")) {
    const keep = q.get("cards")!.split(",");
    cfg.layout.pages = cfg.layout.pages.filter((p: any) => !q.get("page") || p.id === q.get("page"));
    for (const p of cfg.layout.pages) p.cards = keep.map((k) => { const [id, size, design] = k.split(":"); const c = p.cards.find((x: any) => x.id === id) ?? { id, size: "l" }; const o: any = size ? { ...c, size, h: undefined } : { ...c }; if (design) o.look = { ...(o.look ?? {}), design }; return o; });
  }
  // přehlídka vzhledů (`scripts/showcase/designs.mjs`): bohatší ukázková data a pozadí šuplíku z URL
  //   full=1 útrata + předpověď · wx=day|night|rain · bd=<id gradientu> · live=<živé pozadí> · bdcards=auto|solid|tint|glass · empty=1 prázdné stavy
  const empty = !!q.get("empty");
  if (q.get("bd") || q.get("live") || q.get("bdcards")) {
    cfg.appearance.backdrop = { id: q.get("bd") ?? "", live: q.get("live") ?? "", flame: "classic", intensity: 0.6, speed: 1, cards: q.get("bdcards") ?? "auto", custom: [] };
    a = normalizeAppearance(cfg.appearance);
    cfg.appearance = a;
  }
  cfg.media = { ...(cfg.media ?? {}), variant: data.mediaVariant };

  const snap: any = { ...clone(EMPTY_SNAPSHOT), ...clone(data.snap) };
  snap.layout = cfg.layout;
  snap.sections = allSectionsOn();
  snap.mode = q.get("page") ?? cfg.layout.pages[0].id;
  snap.hooks = { state: "full", missing: [], error: null, path: "", hint: null };
  snap.update = { ...snap.update, state: "aktualni", current: "0.11.0" };
  snap.notesEnabled = true;
  snap.smart = { quickKeys: false, dropShortcut: false, tabWheel: true };
  snap.personal = { ...snap.personal, mediaLook: { variant: data.mediaVariant, stripTime: "auto", equalizer: true } };
  if (q.get("nopending")) snap.pending = [];
  if (q.get("ask")) { snap.pending = []; snap.questions = [askFor(now)]; }
  if (q.get("full") && !empty) {
    snap.costs = costsFull(now) as any;
    snap.sessions = [...snap.sessions,
      session("s-orbit", "Opravy synchronizace", "Orbit", "orbit", "working", now, { activity: { phase: "running", tool: "Bash", file: null, message: "npm test", at: now - 3000 } }),
      session("s-pixel", "Přepis dokumentace", "Pixel Docs", "pixel-docs", "idle", now, { updatedAt: now - 9 * 60_000 })] as any;
    const wx = (q.get("wx") ?? "day") as WxKind;
    if (snap.personal) snap.personal = { ...snap.personal, weather: weatherFull(snap.personal.weather?.label || "Praha", now, wx) } as any;
  }
  if (empty) {
    snap.limits = null as any; snap.sessions = []; snap.costs = undefined;
    if (snap.personal) snap.personal = { ...snap.personal, calendar: undefined, weather: { status: "unset", label: "", data: null, fetchedMs: null, error: null, loading: false } } as any;
  }
  snap.view = "hidden";

  const push = () => emitDemo("state", clone(snap));
  const msg = (view: string, morph: boolean) => ({ view, morph, fromW: 360, fromH: 32, toW: drawerWidth(a), toH: 600, maxH: 1400 });

  setDemoHost((cmd, args) => {
    switch (cmd) {
      case "get_config": return clone(cfg);
      case "save_config": return undefined;
      case "get_state": return clone(snap);
      case "set_mode": snap.mode = String(args?.mode); push(); return undefined;
      case "decide": snap.pending = snap.pending.filter((p: any) => p.id !== args?.id); push(); return true;
      case "system_view": return empty ? null : systemFull(now);
      case "media_cover": return COVER;
      case "report_size": parent.postMessage({ ledge: "size", view: args?.view, w: drawerWidth(a), h: args?.h, scheme: document.documentElement.style.colorScheme }, "*"); return undefined;
      case "ui_ready": return undefined;
      case "app_info": return { version: "0.11.0", configPath: "", configDir: "", logPath: "" };
      default: return undefined;
    }
  });

  const open = (morph = false) => {
    emitDemo("view", msg("full", morph));
    if (morph) requestAnimationFrame(() => requestAnimationFrame(() => emitDemo("play", "full")));
  };
  (window as any).__demo = {
    open,
    close: () => emitDemo("closing", { ...msg("hidden", true), toW: 360, toH: 32 }),
    page: (id: string) => { snap.mode = id; push(); },
    appearance: (patch: any) => { a = { ...a, ...patch }; cfg.appearance = a; emitDemo("appearance", clone(a)); },
    style: (id: string, theme?: string) => { a = withStyle(a, id); if (theme) a.theme = theme; cfg.appearance = a; emitDemo("appearance", clone(a)); },
    ask: (on = true) => { snap.pending = on ? [] : snap.pending; snap.questions = on ? [askFor(now)] : []; push(); },
    width: () => drawerWidth(a),
  };
  if (q.get("open")) setTimeout(() => open(false), 60);
}
