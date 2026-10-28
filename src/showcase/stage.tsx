/* @refresh reload */
// Scéna pro snímky a GIFy (`scripts/showcase/stage.html`, kolo 24): vlastní abstraktní tapeta, „horní hrana monitoru“
// s proužkem (HTML náhled `StripLookPreview`) a pod ním skutečný šuplík v iframe (`index.html?demo=1`) nebo Studio
// (`settings.html?demo=1`). Žádná fotka obrazovky — průhlednost šuplíku ukazuje jen tuhle tapetu.
// Scény (`?scene=`): drawer · closed (galerie zavřeného) · open (galerie otevřeného) · studio · pair (zavřený → otevřený).
import "../styles.css";
import "./stage.css";
import { For, Show, createSignal, type JSX } from "solid-js";
import { render } from "solid-js/web";
import StripLookPreview from "../settings/previews/StripLookPreview";
import { STRIP_LOOKS } from "../strip/looks";
import { withStyle } from "../theme/styles";
import { normalizeAppearance } from "../theme/appearance";
import DEFAULTS from "../studio/demo-config.json";
import type { Appearance } from "../types";

const q = new URLSearchParams(location.search);
const get = (k: string, d = "") => q.get(k) ?? d;
const SCENE = get("scene", "drawer");

/** Vzhled pro proužek: styl (nebo výchozí) + motiv. */
function appearanceOf(style: string, theme: string): Appearance {
  const a = style ? withStyle(DEFAULTS.appearance, style) : normalizeAppearance(DEFAULTS.appearance);
  return theme ? { ...a, theme: theme as any } : a;
}

/** Proužek u horní hrany, zvětšený (`zoom`), na průhledném pozadí. */
function Strip(p: { look: string; state: string; a: Appearance; zoom?: number; width?: number }) {
  return (
    <div class="sx-strip" style={{ width: `${p.width ?? 520}px`, zoom: String(p.zoom ?? 1.5) }}>
      <StripLookPreview look={p.look as any} state={p.state as any} a={p.a} opacity={0.92} compose={DEFAULTS.strip.compose} />
    </div>
  );
}

/** Parametry pro iframe šuplíku = parametry scény bez vlastních. */
function drawerSrc(extra: Record<string, string> = {}): string {
  const u = new URLSearchParams();
  for (const [k, v] of q) if (!["scene", "look", "state", "cap", "vw", "vh"].includes(k)) u.set(k, v);
  for (const [k, v] of Object.entries(extra)) u.set(k, v);
  u.set("demo", "1");
  u.set("open", "1");
  return `../../index.html?${u}`;
}

function Wall(p: { children: JSX.Element; k?: string; overlap?: boolean }) {
  return (
    <div class="sx-wall" data-k={p.k} classList={{ "sx-overlap": !!p.overlap }}>
      {p.children}
    </div>
  );
}

function Drawer(p: { src: string; w?: number; h?: number; scale?: number; id?: string; fixed?: boolean }) {
  let f!: HTMLIFrameElement;
  const sync = (e: MessageEvent) => {
    if (e.source !== f.contentWindow || e.data?.ledge !== "size") return;
    f.style.width = `${e.data.w}px`;
    if (!p.fixed) f.style.height = `${Math.ceil(e.data.h) + 18}px`;
    f.style.colorScheme = e.data.scheme || "dark"; // shoda s vloženým dokumentem, jinak je iframe neprůhledný
    f.dataset.ready = "1";
  };
  window.addEventListener("message", sync);
  return (
    <iframe ref={f} id={p.id} class="sx-drawer" src={p.src} style={{ width: `${p.w ?? 1080}px`, height: `${p.h ?? 700}px`, zoom: p.scale ? String(p.scale) : undefined }} />
  );
}

function DrawerScene() {
  const a = appearanceOf(get("style"), get("theme"));
  return (
    <Wall overlap={q.has("overlap")}>
      <Show when={get("look")}><Strip look={get("look", "island")} state={get("state", "work")} a={a} /></Show>
      <Drawer src={drawerSrc()} id="main" />
    </Wall>
  );
}

function ClosedGallery() {
  const ids = get("looks", "classic,island,dots,line,icons,clock,corner,custom").split(",");
  const a = appearanceOf(get("style"), get("theme"));
  const state = get("state", "work");
  return (
    <Wall k="closed">
      <For each={ids}>
        {(id) => (
          <figure class="sx-cell">
            <div class="sx-edge"><Strip look={id} state={state} a={a} zoom={1.6} width={225} /></div>
            <figcaption>{STRIP_LOOKS.find((l) => l.id === id)?.cs ?? id}</figcaption>
          </figure>
        )}
      </For>
    </Wall>
  );
}

const STYLE_NAMES: Record<string, string> = { classic: "Klasický", glass: "Sklo", terminal: "Terminál", paper: "Papír", neon: "Neon", oled: "OLED", notch: "Notch", aurora: "Aurora", warm: "Teplý", mono: "Mono" };

function OpenGallery() {
  const items = get("items", "classic,glass,terminal,paper,neon,oled").split(",");
  const scale = Number(get("scale", "0.5"));
  const cols = Number(get("cols", "3"));
  return (
    <Wall k="open">
      <div class="sx-gridbox" style={{ "grid-template-columns": `repeat(${cols}, auto)` }}>
        <For each={items}>
          {(it) => {
            const [style, theme] = it.split(":");
            return (
              <figure class="sx-cell">
                <div class="sx-clip" style={{ width: `${1080 * scale}px`, height: `${Number(get("h", "640")) * scale}px` }}>
                  <Drawer fixed src={drawerSrc({ style, ...(theme ? { theme } : {}), w: "1080" })} w={1080} h={Number(get("h", "640"))} scale={scale} />
                </div>
                <figcaption>{STYLE_NAMES[style] ?? style}{theme ? ` · ${theme}` : ""}</figcaption>
              </figure>
            );
          }}
        </For>
      </div>
    </Wall>
  );
}

function StudioScene() {
  const hash = get("hash");
  const src = `../../settings.html?demo=1${get("q") ? `&q=${encodeURIComponent(get("q"))}` : ""}${hash ? `#${hash}` : ""}`;
  return (
    <Wall>
      <iframe id="main" class="sx-studio" src={src} />
    </Wall>
  );
}

function PairScene() {
  const a = appearanceOf(get("style"), get("theme"));
  return (
    <Wall k="pair">
      <div class="sx-half">
        <div class="sx-mon"><div class="sx-edge"><Strip look={get("look", "island")} state={get("state", "work")} a={a} zoom={1.7} width={380} /></div></div>
        <figcaption>Zavřený</figcaption>
      </div>
      <div class="sx-arrow">→</div>
      <div class="sx-half">
        <div class="sx-mon"><Drawer fixed src={drawerSrc({ w: "1080" })} w={1080} h={Number(get("h", "760"))} scale={0.6} id="main" /></div>
        <figcaption>Otevřený</figcaption>
      </div>
    </Wall>
  );
}

/** Jeden proužek přes celou šířku s popiskem; řídí ho Playwright (`window.__stage.set`) — GIF „proužek prochází vzhledy“. */
function StripScene() {
  const [look, setLook] = createSignal(get("look", "island"));
  const [state, setState] = createSignal(get("state", "idle"));
  const [theme, setTheme] = createSignal(get("theme", ""));
  (window as any).__stage = { set: (v: { look?: string; state?: string; theme?: string }) => { if (v.look) setLook(v.look); if (v.state) setState(v.state); if (v.theme !== undefined) setTheme(v.theme); } };
  return (
    <Wall k="strip">
      <div class="sx-strip-big"><Strip look={look()} state={state()} a={appearanceOf(get("style"), theme())} zoom={2.2} width={520} /></div>
      <figcaption class="sx-cap">{STRIP_LOOKS.find((l) => l.id === look())?.cs ?? look()} · {({ idle: "klid", work: "Claude pracuje", wait: "čeká na tebe" } as Record<string, string>)[state()] ?? state()}</figcaption>
    </Wall>
  );
}

const scenes: Record<string, () => JSX.Element> = { drawer: DrawerScene, closed: ClosedGallery, open: OpenGallery, studio: StudioScene, pair: PairScene, strip: StripScene };
document.documentElement.dataset.scene = SCENE;
render(() => (scenes[SCENE] ?? DrawerScene)(), document.getElementById("root") as HTMLElement);
