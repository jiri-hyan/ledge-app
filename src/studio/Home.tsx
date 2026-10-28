// Domov Ledge Studia „Tvůj Ledge“ (kolo 24): velká klikací kresba TVÉHO Ledge — horní hrana monitoru se
// skutečným vzhledem proužku, pod ní otevřený šuplík se skutečnými záložkami a kartami (`layout.pages`,
// schematicky, v aktuálním motivu), plocha s lístkem a hlavní panel. Každá část je zóna s číslem stanice:
// najetí ji zvýrazní a řekne, co se tam nastavuje, klik = do části (`anatomy.ts` `ZONES`, `homeModel`).
// Vedle „Na co nezapomenout“ (jen ze skutečného stavu — nic = karta se neukáže) a „Rychle“ (pár nejčastějších
// voleb rovnou jako ovladače). Nahrazuje stránku Doporučené. Žádný časovač ani animace v klidu.
import { For, Show, createMemo, createSignal } from "solid-js";
import { t } from "../i18n";
import { lbl } from "../theme/label";
import type { Snapshot } from "../types";
import { cfg, useSettings } from "../settings/ctx";
import { normalizeAppearance } from "../theme/appearance";
import { normalizeLayout } from "../layout/model";
import { cardName, pageName } from "../layout/text";
import { normLook } from "../settings/panels/AppearanceClosed";
import { normHover } from "../settings/panels/AppearanceHover";
import { normShape, STRIP_LOOKS } from "../strip/looks";
import { STRIP_OPACITY } from "../settings/strip";
import { HOVER_MODES } from "../hover/modes";
import StripLookPreview from "../settings/previews/StripLookPreview";
import { Seg, Select, Toggle } from "../settings/fields/controls";
import { fieldLabel } from "../settings/labels";
import { bubble } from "../pkgctx/BubbleNotice";
import { partById, visibleParts, type PartId } from "./parts";
import { homeModel, reminders, type Reminder } from "./anatomy";
import { IconGuide, IconClose, PartIcon } from "./icons";
import { NEWS, TIPS, newsState } from "./tips";

const Badge = (q: { n: number }) => <i class="sh-badge" aria-hidden="true">{q.n}</i>;

const REMINDER_TEXT: Record<string, () => [string, string, string]> = {
  hooks: () => [t("settings.studio.rem.hooks", "Claude Code ještě není propojený"), t("settings.studio.rem.hooks.text", "Bez propojení Ledge nevidí, na čem Claude pracuje, ani jeho žádosti o povolení."), t("settings.studio.rem.hooks.go", "Propojit")],
  update: () => [t("settings.studio.rem.update", "Je tu nová verze"), t("settings.studio.rem.update.text", "Podívej se, co je nového, a nainstaluj ji jedním klikem."), t("settings.studio.rem.update.go", "Ukázat")],
  city: () => [t("settings.studio.rem.city", "Počasí nemá město"), t("settings.studio.rem.city.text", "Karta počasí je v šuplíku, ale bez města nic neukáže."), t("settings.studio.rem.city.go", "Vybrat město")],
  calendar: () => [t("settings.studio.rem.calendar", "Kalendář není připojený"), t("settings.studio.rem.calendar.text", "Karta kalendáře je v šuplíku, ale nemá odkud brát schůzky."), t("settings.studio.rem.calendar.go", "Připojit")],
  project: () => [t("settings.studio.rem.project", "Zatím žádný projekt"), t("settings.studio.rem.project.text", "Přidej složku s projektem — jak ho spustit, Ledge zjistí sám."), t("settings.studio.rem.project.go", "Přidat projekt")],
  autostart: () => [t("settings.studio.rem.autostart", "Ledge se nespouští s Windows"), t("settings.studio.rem.autostart.text", "Po restartu ho budeš muset pustit ručně."), t("settings.studio.rem.autostart.go", "Zapnout")],
};

const TIP_TEXT: Record<string, () => [string, string]> = {
  palette: () => [t("settings.start.tip.palette", "Paleta příkazů"), t("settings.start.tip.palette.text", "Ctrl+K nebo 🔍 v hlavičce: najde projekty, akce, poznámky i volby Nastavení, spočítá příklad a převede jednotky.")],
  edit: () => [t("settings.start.tip.edit", "Úpravy přímo v šuplíku"), t("settings.start.tip.edit.text", "Tužka v hlavičce šuplíku: karty přetáhneš, změníš jim velikost, přidáš z katalogu a založíš vlastní záložky.")],
  card: () => [t("settings.start.tip.card", "Nabídka karty"), t("settings.start.tip.card.text", "Tři tečky v rohu každé karty: skrýt kartu nebo jen její část, nebo rovnou upravit šuplík.")],
  search: () => [t("settings.start.tip.search", "Hledání v Nastavení"), t("settings.start.tip.search.new", "Najde každou volbu, i když nevíš, jak se jmenuje, a bez háčků a čárek.")],
};
const NEWS_TEXT: Record<string, () => [string, string]> = {
  studio: () => [t("settings.start.new.studio", "Nastavení jako Ledge Studio"), t("settings.start.new.studio.text", "Samostatné okno: části podle toho, jak Ledge vypadá, změny platí hned a každou jde vrátit.")],
  motion: () => [t("settings.start.new.motion", "Nové animace otevírání"), t("settings.start.new.motion.text", "Vyber, jak šuplík vyjede — nebo žádnou animaci.")],
  media: () => [t("settings.start.new.media", "Přehrávač v několika podobách"), t("settings.start.new.media.text", "Karta Právě hraje jako řádek, karta, velký obal nebo vinyl.")],
};

const SEEN_KEY = "ledge.studio.seenVersion";
const seenGet = () => { try { return localStorage.getItem(SEEN_KEY); } catch { return null; } };
const seenSet = (v: string) => { try { localStorage.setItem(SEEN_KEY, v); } catch { /* jen pohodlí */ } };

export default function Home(p: { s: Snapshot; go: (target: string) => void; modified: Record<PartId, number>; onGuide: () => void; guideDone: number }) {
  const c = useSettings();
  const [hover, setHover] = createSignal<PartId | null>(null);
  const a = () => normalizeAppearance(cfg.appearance);
  const L = createMemo(() => normalizeLayout(cfg.layout));
  const home = createMemo(() => homeModel(L(), p.s.mode));
  const rems = createMemo(() => reminders({ cfg, hooks: p.s.hooks, update: p.s.update, packaged: !!bubble() }));
  // „Co je nového“: jen po přechodu z nižší verze, zavřít jde (zapíše se současná verze)
  const current = () => p.s.update?.current;
  const news0 = newsState(seenGet(), current());
  if (news0 === "first" && current()) seenSet(current()!);
  const [newsOpen, setNewsOpen] = createSignal(news0 === "show");
  const closeNews = () => (setNewsOpen(false), current() && seenSet(current()!));
  const first = (part: PartId) => home().cards.findIndex((x) => x.part === part);

  /** zóna kresby: zvýraznění, popisek a klik (vnořené zóny klik nepropustí výš) */
  const zone = (part: PartId) => ({
    "data-zone": part,
    role: "button" as const,
    tabIndex: 0,
    "aria-label": lbl(partById(part)!.name),
    "data-lit": hover() === part ? "" : undefined,
    onPointerEnter: () => setHover(part),
    onPointerLeave: () => setHover((h) => (h === part ? null : h)),
    onFocus: () => setHover(part),
    onClick: (e: MouseEvent) => (e.stopPropagation(), p.go(part)),
    onKeyDown: (e: KeyboardEvent) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), e.stopPropagation(), p.go(part)),
  });
  const shown = () => partById(hover());

  const fix = (r: Reminder) => {
    if (r.fix === "autostart") return c.set("autostart", true);
    p.go(r.go);
  };

  return (
    <div class="sh">
      <section class="sh-stage">
        <div class="sh-art" classList={{ hovering: !!hover() }} data-layout={a().layout}>
          <div class="sh-wall" />
          {/* 1 · proužek: skutečný vzhled zavřeného Ledge */}
          <div class="sh-strip" {...zone("pruzek")}>
            <StripLookPreview look={normLook(cfg.strip?.look)} state="work" a={a()} opacity={cfg.strip?.opacity ?? STRIP_OPACITY} shape={normShape(cfg.strip?.lookShape)} compose={cfg.strip?.compose} />
            <Badge n={1} />
          </div>
          {/* 2 · otevírání: kurzor k hraně */}
          <div class="sh-edge" {...zone("otevirani")}>
            <svg viewBox="0 0 60 60" aria-hidden="true"><path class="sh-trail" d="M52 54 C 40 40, 30 26, 22 10" /><path class="sh-cursor" d="M16 2 l0 17 4.5-4.2 3.2 7 3-1.3-3.2-6.8 6.2-.4z" /></svg>
            <Badge n={2} />
          </div>
          {/* 3 · vzhled šuplíku: rám; 4 · obsah: záložky a karty; 5 · Claude; 6 · projekty */}
          <div class="sh-drawer" {...zone("vzhled")}>
            <Badge n={3} />
            <div class="sh-tabs" {...zone("obsah")}>
              <For each={home().tabs}>{(tb) => <span classList={{ on: tb.on }}>{pageName(L().pages.find((x) => x.id === tb.id)!)}</span>}</For>
              <Badge n={4} />
            </div>
            <div class="sh-grid">
              <For each={home().cards}>
                {(cd, i) => (
                  <div class="sh-card" style={{ "--span": String(cd.span) }} data-part={cd.part} {...zone(cd.part)}>
                    <b>{cardName(cd.id as any)}</b>
                    <i /><i class="short" />
                    <Show when={(cd.part === "claude" || cd.part === "projekty") && first(cd.part) === i()}><Badge n={cd.part === "claude" ? 5 : 6} /></Show>
                  </div>
                )}
              </For>
              <Show when={home().more}><span class="sh-more">+{home().more}</span></Show>
            </div>
            <div class="sh-foot">
              <Show when={first("claude") < 0}><span class="sh-ghost" {...zone("claude")}><PartIcon icon="claude" />{t("settings.studio.p.claude", "Claude")}<Badge n={5} /></span></Show>
              <Show when={first("projekty") < 0}><span class="sh-ghost" {...zone("projekty")}><PartIcon icon="projects" />{t("settings.studio.p.projekty", "Projekty a akce")}<Badge n={6} /></span></Show>
            </div>
          </div>
          {/* 7 · poznámky a plocha */}
          <div class="sh-desk" {...zone("poznamky")}>
            <span class="sh-note"><i /><i /><i class="short" /></span>
            <span class="sh-widget"><b>14:32</b><i /></span>
            <Badge n={7} />
          </div>
          {/* 8 · aplikace: hlavní panel s ikonou Ledge */}
          <div class="sh-taskbar" {...zone("aplikace")}>
            <span class="sh-tray"><i class="sh-logo" /><i /><i /></span>
            <Badge n={8} />
          </div>
        </div>
        <div class="sh-caption" aria-live="polite">
          <Show when={shown()} fallback={<p>{t("settings.studio.home.pick", "Najeď na část svého Ledge — řeknu ti, co se tam nastavuje. Klik tě tam vezme.")}</p>}>
            {(pt) => (
              <p>
                <b>{pt().n} · {lbl(pt().name)}</b> — {lbl(pt().line)}
                <Show when={p.modified[pt().id]}> <em>{t("settings.studio.home.changed", "{n} upraveno", { n: p.modified[pt().id] })}</em></Show>
              </p>
            )}
          </Show>
        </div>
        <button class="sh-guide" onClick={p.onGuide}>
          <span class="sh-guide-ico"><IconGuide /></span>
          <span>
            <b>{p.guideDone ? t("settings.studio.home.guideGo", "Pokračovat v průvodci") : t("settings.studio.home.guide", "Projít Ledge krok za krokem")}</b>
            <small>{t("settings.studio.home.guideText", "8 částí po pár otázkách, v pořadí, jak se s Ledge potkáváš. Všechno platí hned a jde vrátit.")}</small>
          </span>
          <Show when={p.guideDone}><em>{p.guideDone} / 8</em></Show>
        </button>
      </section>

      <aside class="sh-side">
        <Show when={newsOpen()}>
          <section class="sh-block sh-news">
            <h3>{t("settings.start.news", "Co je nového")}<button class="sh-x" onClick={closeNews} aria-label={t("settings.studio.news.close", "Zavřít")} title={t("settings.studio.news.close", "Zavřít")}><IconClose /></button></h3>
            <For each={NEWS}>
              {(x) => (
                <article class="sh-tip">
                  <kbd>{x.k}</kbd>
                  <div><b>{NEWS_TEXT[x.id]()[0]}</b><small>{NEWS_TEXT[x.id]()[1]}</small></div>
                  <Show when={x.go}><button class="btn" onClick={() => p.go(x.go!)}>{t("settings.start.show", "Ukázat")}</button></Show>
                </article>
              )}
            </For>
          </section>
        </Show>
        <Show when={rems().length}>
          <section class="sh-block">
            <h3>{t("settings.studio.home.remember", "Na co nezapomenout")}</h3>
            <For each={rems()}>
              {(r) => (
                <article class="sh-rem" data-part={r.part}>
                  <div>
                    <b>{REMINDER_TEXT[r.id]()[0]}</b>
                    <small>{REMINDER_TEXT[r.id]()[1]}</small>
                  </div>
                  <button class="btn pri" onClick={() => fix(r)}>{REMINDER_TEXT[r.id]()[2]}</button>
                </article>
              )}
            </For>
          </section>
        </Show>
        <section class="sh-block sh-quick">
          <h3>{t("settings.studio.home.quick", "Rychle")}</h3>
          <label class="sh-q"><span>{fieldLabel("strip.look")}</span>
            <Select label={fieldLabel("strip.look")} options={STRIP_LOOKS.map((x) => ({ v: x.id as string, label: t(x.key, x.cs) }))} value={normLook(cfg.strip?.look)} onChange={(v) => c.set("strip.look", v)} />
          </label>
          <label class="sh-q"><span>{fieldLabel("drawer.hover")}</span>
            <Select label={fieldLabel("drawer.hover")} options={HOVER_MODES.map((x) => ({ v: x.id as string, label: t(x.key, x.cs) }))} value={normHover(cfg.drawer?.hover)} onChange={(v) => c.set("drawer.hover", v)} />
          </label>
          <div class="sh-q"><span>{fieldLabel("ui.lang")}</span>
            <Seg path="ui.lang" label={fieldLabel("ui.lang")} options={[{ v: "cs", label: t("settings.studio.lang.cs", "Čeština") }, { v: "en", label: "English" }]} />
          </div>
          <div class="sh-q"><span>{fieldLabel("autostart")}</span><Toggle path="autostart" label={fieldLabel("autostart")} disabled={!!bubble()} /></div>
          <div class="sh-q"><span>{fieldLabel("shelf.enabled")}</span><Toggle path="shelf.enabled" label={fieldLabel("shelf.enabled")} /></div>
        </section>
        <section class="sh-block sh-parts">
          <h3>{t("settings.studio.home.all", "Všechny části")}</h3>
          <For each={visibleParts(cfg)}>
            {(pt) => (
              <button class="sh-part" onClick={() => p.go(pt.id)} onPointerEnter={() => setHover(pt.id)} onPointerLeave={() => setHover(null)}>
                <i class="sh-n">{pt.n}</i>
                <PartIcon icon={pt.icon} />
                <span><b>{lbl(pt.name)}</b><small>{lbl(pt.line)}</small></span>
              </button>
            )}
          </For>
        </section>
        <section class="sh-block sh-tips">
          <h3>{t("settings.rec.tips", "Dobré vědět")}</h3>
          <For each={TIPS}>
            {(x) => (
              <article class="sh-tip">
                <kbd>{x.k}</kbd>
                <div><b>{TIP_TEXT[x.id]()[0]}</b><small>{TIP_TEXT[x.id]()[1]}</small></div>
                <Show when={x.go}><button class="btn" onClick={() => p.go(x.go!)}>{t("settings.start.show", "Ukázat")}</button></Show>
              </article>
            )}
          </For>
        </section>
      </aside>
    </div>
  );
}
