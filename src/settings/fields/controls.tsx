// Ovládací prvky Nastavení, všechny stejného stylu (settings.css, jen tokeny): přepínač, segmenty,
// výběr, posuvník s hodnotou, číslo, text, barva, cesta s „Vybrat…“ (nativní dialog) a seznam štítků.
// S `path` čtou a mění config přes `useSettings()`, bez něj fungují přes `value` + `onChange`.
import { createSignal, For, Show } from "solid-js";
import { api, type PickKind } from "../../api";
import { t } from "../../i18n";
import { sayError } from "../../ui/toast";
import { useSettings } from "../ctx";
import { normHex } from "../model";

type Bind<T> = { path?: string; value?: T; onChange?: (v: T) => void };

function bind<T>(p: Bind<T>) {
  const c = useSettings();
  return {
    get: (): T => (p.path ? c.get(p.path) : p.value) as T,
    put: (v: T) => (p.path ? c.set(p.path, v) : p.onChange?.(v)),
    bad: () => !!(p.path && c.error(p.path)),
  };
}

export function Toggle(p: Bind<boolean> & { label: string; disabled?: boolean }) {
  const b = bind(p);
  return (
    <button
      class="st-switch"
      classList={{ on: !!b.get() }}
      role="switch"
      aria-checked={!!b.get()}
      aria-label={p.label}
      disabled={p.disabled}
      onClick={() => b.put(!b.get())}
    />
  );
}

export type Opt<T> = { v: T; label: string; hint?: string };

/** Segmenty (2–4 volby): radio skupina, šipky mezi volbami. */
export function Seg<T extends string | number>(p: Bind<T> & { options: Opt<T>[]; label: string }) {
  const b = bind(p);
  const key = (e: KeyboardEvent, i: number) => {
    const d = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
    if (!d) return;
    e.preventDefault();
    const n = p.options[(i + d + p.options.length) % p.options.length];
    b.put(n.v);
    ((e.currentTarget as HTMLElement).parentElement?.children[p.options.indexOf(n)] as HTMLElement | undefined)?.focus();
  };
  return (
    <div class="st-seg" role="radiogroup" aria-label={p.label}>
      <For each={p.options}>
        {(o, i) => (
          <button role="radio" aria-checked={b.get() === o.v} tabIndex={b.get() === o.v ? 0 : -1} classList={{ on: b.get() === o.v }} title={o.hint} onClick={() => b.put(o.v)} onKeyDown={(e) => key(e, i())}>
            {o.label}
          </button>
        )}
      </For>
    </div>
  );
}

export function Select<T extends string>(p: Bind<T> & { options: Opt<T>[]; label: string }) {
  const b = bind(p);
  return (
    <select aria-label={p.label} value={b.get() ?? ""} classList={{ bad: b.bad() }} onChange={(e) => b.put(e.currentTarget.value as T)}>
      <For each={p.options}>{(o) => <option value={o.v}>{o.label}</option>}</For>
    </select>
  );
}

/** Posuvník s hodnotou a jednotkou. `fmt` = jak hodnotu ukázat (např. ms → s). */
export function Slider(p: Bind<number> & { min: number; max: number; step?: number; unit?: string; label: string; fmt?: (v: number) => string }) {
  const b = bind(p);
  const pct = () => ((Math.min(p.max, Math.max(p.min, b.get() ?? p.min)) - p.min) / (p.max - p.min)) * 100;
  return (
    <div class="st-slider">
      <input
        type="range"
        min={p.min}
        max={p.max}
        step={p.step ?? 1}
        value={b.get() ?? p.min}
        aria-label={p.label}
        style={{ "--p": `${pct()}%` }}
        onInput={(e) => b.put(Number(e.currentTarget.value))}
      />
      <output>{p.fmt ? p.fmt(b.get()) : `${b.get()}${p.unit ? ` ${p.unit}` : ""}`}</output>
    </div>
  );
}

export function Num(p: Bind<number | null> & { min?: number; max?: number; step?: number; unit?: string; label: string; placeholder?: string; nullable?: boolean; width?: string }) {
  const b = bind(p);
  /** `live` = při psaní: zapsat jen hotové číslo („50.“ by se přepsalo na „50“ a tečka zmizela) */
  const commit = (raw: string, live: boolean) => {
    const s = raw.trim().replace(",", ".");
    if (!s) return live ? undefined : b.put(p.nullable ? null : (p.min ?? 0));
    const n = Number(s);
    if (!Number.isFinite(n) || (live && String(n) !== s)) return;
    b.put(n);
  };
  return (
    <span class="st-num">
      <input
        type="number"
        min={p.min}
        max={p.max}
        step={p.step ?? 1}
        aria-label={p.label}
        placeholder={p.placeholder}
        style={{ width: p.width }}
        classList={{ bad: b.bad() }}
        value={b.get() ?? ""}
        onInput={(e) => commit(e.currentTarget.value, true)}
        onChange={(e) => commit(e.currentTarget.value, false)}
      />
      <Show when={p.unit}><span class="unit">{p.unit}</span></Show>
    </span>
  );
}

export function Text(p: Bind<string | null> & { label: string; placeholder?: string; mono?: boolean; grow?: boolean; nullable?: boolean; maxLength?: number }) {
  const b = bind(p);
  return (
    <input
      type="text"
      class="st-text"
      classList={{ mono: p.mono, grow: p.grow, bad: b.bad() }}
      aria-label={p.label}
      placeholder={p.placeholder}
      maxLength={p.maxLength}
      spellcheck={false}
      value={b.get() ?? ""}
      onInput={(e) => {
        const v = e.currentTarget.value;
        b.put(p.nullable && !v.trim() ? null : v);
      }}
      onChange={(e) => {
        const v = e.currentTarget.value.trim();
        b.put(p.nullable && !v ? null : v);
      }}
    />
  );
}

const SWATCHES = ["#a78bfa", "#60a5fa", "#2dd4bf", "#4ade80", "#fbbf24", "#fb923c", "#f87171", "#f472b6", "#94a3b8"];

/** Barva: vzorky, vlastní (nativní výběr) a hex. `allowEmpty` = „žádná“ (null / ""). */
export function Color(p: Bind<string | null> & { label: string; allowEmpty?: boolean; empty?: string | null }) {
  const b = bind(p);
  const [hex, setHex] = createSignal<string | null>(null);
  const cur = () => b.get() || "";
  return (
    <div class="st-color" role="group" aria-label={p.label}>
      <Show when={p.allowEmpty}>
        <button class="sw none" classList={{ on: !cur() }} title={t("settings.ui.color.none", "Žádná (výchozí)")} aria-label={t("settings.ui.color.none", "Žádná (výchozí)")} onClick={() => b.put((p.empty ?? null) as any)} />
      </Show>
      <For each={SWATCHES}>
        {(c) => <button class="sw" classList={{ on: cur().toLowerCase() === c }} style={{ "--sw": c }} aria-label={c} title={c} onClick={() => b.put(c)} />}
      </For>
      <label class="sw custom" title={t("settings.ui.color.custom", "Vlastní barva")} style={{ "--sw": normHex(cur()) ?? "transparent" }}>
        <input type="color" value={normHex(cur()) ?? "#a78bfa"} onInput={(e) => b.put(e.currentTarget.value)} aria-label={t("settings.ui.color.custom", "Vlastní barva")} />
      </label>
      <input
        type="text"
        class="st-text mono hex"
        aria-label={`${p.label} (hex)`}
        maxLength={7}
        spellcheck={false}
        classList={{ bad: hex() !== null && !normHex(hex()!) && hex() !== "" }}
        value={hex() ?? cur()}
        placeholder="#rrggbb"
        onInput={(e) => {
          const v = e.currentTarget.value;
          setHex(v);
          const n = normHex(v);
          if (n) b.put(n);
          else if (!v.trim() && p.allowEmpty) b.put((p.empty ?? null) as any);
        }}
        onBlur={() => setHex(null)}
      />
    </div>
  );
}

/** Cesta: text + „Vybrat…“ (nativní dialog). `map` = úprava vybrané cesty (např. relativně k projektu). */
export function PathPick(p: Bind<string | null> & { label: string; kind: PickKind; placeholder?: string; start?: string | null; map?: (v: string) => string; nullable?: boolean }) {
  const b = bind(p);
  const [busy, setBusy] = createSignal(false);
  async function pick() {
    setBusy(true);
    try {
      const got = await api.pickPath(p.kind, p.label, p.start ?? undefined);
      if (got) b.put(p.map ? p.map(got) : got);
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
    }
  }
  return (
    <div class="st-path">
      <Text path={p.path} value={p.value} onChange={p.onChange} label={p.label} placeholder={p.placeholder} mono grow nullable={p.nullable} />
      <button class="mini" disabled={busy()} onClick={pick}>{t("settings.ui.pick", "Vybrat…")}</button>
    </div>
  );
}

/** Seznam krátkých textů jako štítky (procesy, argumenty): × odebere, Enter přidá. */
export function Chips(p: Bind<string[]> & { label: string; placeholder?: string; normalize?: (s: string) => string }) {
  const b = bind(p);
  const [draft, setDraft] = createSignal("");
  const list = () => b.get() ?? [];
  const add = () => {
    const v = (p.normalize ?? ((s: string) => s.trim()))(draft());
    if (v && !list().some((x) => x.toLowerCase() === v.toLowerCase())) b.put([...list(), v]);
    setDraft("");
  };
  return (
    <div class="st-chips" role="group" aria-label={p.label}>
      <For each={list()}>
        {(x, i) => (
          <span class="st-chip">
            {x}
            <button aria-label={t("settings.ui.chip.remove", "Odebrat {x}", { x })} onClick={() => b.put(list().filter((_, j) => j !== i()))}>×</button>
          </span>
        )}
      </For>
      <input
        type="text"
        class="st-text chip-in"
        placeholder={p.placeholder}
        value={draft()}
        spellcheck={false}
        aria-label={p.label}
        onInput={(e) => setDraft(e.currentTarget.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === ",") {
            e.preventDefault();
            add();
          } else if (e.key === "Backspace" && !draft() && list().length) b.put(list().slice(0, -1));
        }}
        onBlur={add}
      />
    </div>
  );
}
