// Editor jednoho projektu (rozbalený řádek v panelu Projekty): název, id, druh, složka/remote/podsložka,
// příkaz a port (dev) nebo program (app), barva, monogram, logo s náhledem, odkazy. Všechna pole píšou
// do `projects.<i>.*` a chyby validace z Rustu se ukážou přímo u pole (`FieldErr`).
import { createEffect, createResource, createSignal, For, on, Show } from "solid-js";
import { api } from "../../api";
import { t } from "../../i18n";
import { base, useSettings } from "../ctx";
import { Chips, Color, Num, PathPick, Seg, Text, Toggle } from "../fields/controls";
import { FieldErr } from "../fields/Row";
import { relativeTo } from "../model";
import { idChanged, linkProblem } from "../projectAdd";
import { useTone } from "../../ui/iconTone";

/** Logo projektu: z dlaždice (snapshot), když se pole loga nezměnila; jinak náhled z jádra. */
export function ProjectLogo(p: { project: any; savedLogo?: string; size?: number }) {
  const key = () => {
    const x = p.project;
    return JSON.stringify([x.id, x.kind, x.path, x.remote, x.subdir, x.logo, x.exe, x.appName, x.aumid]);
  };
  // náhled se ptá s malým zpožděním (psaní cesty), a jen když dlaždice logo nemá
  const [debounced, setDebounced] = createSignal(key());
  let timer: number | undefined;
  createEffect(on(key, (k) => {
    clearTimeout(timer);
    timer = window.setTimeout(() => setDebounced(k), 400);
  }, { defer: true }));
  const [preview] = createResource(() => (p.savedLogo ? null : debounced()), () => api.logoPreview(JSON.parse(JSON.stringify(p.project))).catch(() => null));
  const src = () => p.savedLogo ?? preview() ?? undefined;
  const mono = () => (p.project.monogram || p.project.name || "?").slice(0, 2);
  const tone = useTone(src);
  return (
    <span class="st-logo" style={{ "--acc": p.project.accent || "var(--violet)", width: `${p.size ?? 30}px`, height: `${p.size ?? 30}px` }}>
      <Show when={src()} fallback={<span class="mono-l">{mono()}</span>}>{(s) => <img classList={{ "tone-dark": tone() === "dark", "tone-light": tone() === "light" }} src={s()} alt="" />}</Show>
    </span>
  );
}

export function ProjectEditor(p: { i: number; notes?: string[] }) {
  const c = useSettings();
  const pr = () => c.cfg.projects[p.i];
  const at = (f: string) => `projects.${p.i}.${f}`;
  // id, se kterým se editor otevřel: změna u už uloženého projektu osiří poznámky, vzhled dlaždice a logo
  const startId = pr().id as string;
  const idWarn = () => {
    try {
      return idChanged(JSON.parse(base() || "{}").projects ?? [], startId, pr().id);
    } catch {
      return false;
    }
  };
  const L = (label: string, hint?: string) => (
    <>
      <span class="st-sub-l">{label}</span>
      <Show when={hint}><small>{hint}</small></Show>
    </>
  );
  /** složka projektu pro relativní cesty (exe, logo) — jen když je vyplněná v configu */
  const dir = () => {
    const x = pr();
    return x.path ? (x.subdir ? `${x.path}\\${x.subdir}` : x.path) : null;
  };
  return (
    <div class="st-proj-edit">
      <Show when={p.notes?.length}>
        <div class="st-info">
          <For each={p.notes}>{(n) => <div>{n}</div>}</For>
        </div>
      </Show>
      <div class="st-form">
        <label class="st-sub">{L(t("settings.proj.name", "Název"))}<Text path={at("name")} label={t("settings.proj.name", "Název")} maxLength={60} /><FieldErr path={at("name")} /></label>
        <label class="st-sub">{L(t("settings.proj.id", "Id"), t("settings.proj.id.hint", "Krátké a jedinečné (kebab-case). Používá ho API a poznámky."))}<Text path={at("id")} label={t("settings.proj.id", "Id")} mono maxLength={64} /><FieldErr path={at("id")} /><Show when={idWarn()}><small class="warn" role="status">{t("settings.proj.id.warn", "Pozor: po změně id se ztratí poznámky, vzhled dlaždice a logo uložené pod starým id.")}</small></Show></label>
        <div class="st-sub wide">
          {L(t("settings.proj.kind", "Druh"))}
          <Seg
            path={at("kind")}
            label={t("settings.proj.kind", "Druh")}
            options={[
              { v: "dev", label: t("settings.proj.kind.dev", "Web / dev server") },
              { v: "app", label: t("settings.proj.kind.app", "Desktopová aplikace") },
            ]}
          />
        </div>
        <label class="st-sub wide">
          {L(t("settings.proj.path", "Složka"), t("settings.proj.path.hint", "Prázdné = dohledá se podle git remote v „Kde hledat repa“ — funguje stejně na každém PC."))}
          <PathPick path={at("path")} kind="folder" nullable label={t("settings.proj.path", "Složka")} placeholder={t("settings.proj.path.ph", "dohledat podle remote")} />
          <FieldErr path={at("path")} />
        </label>
        <label class="st-sub">{L(t("settings.proj.remote", "Git remote"), t("settings.proj.remote.hint", "https://…, git@… nebo owner/repo"))}<Text path={at("remote")} nullable mono label={t("settings.proj.remote", "Git remote")} placeholder="owner/repo" /><FieldErr path={at("remote")} /></label>
        <div class="st-sub wide">
          <span class="st-check"><Toggle path={at("muteDirty")} label={t("settings.proj.muteDirty", "Ztišit upozornění na neuložený git")} /> {t("settings.proj.muteDirty", "Ztišit upozornění na neuložený git")}<small> — {t("settings.proj.muteDirty.hint", "nezobrazovat v proužku a v hlavičce šuplíku")}</small></span>
        </div>
        <label class="st-sub">{L(t("settings.proj.subdir", "Podsložka"), t("settings.proj.subdir.hint", "Kde se příkaz spouští, např. web"))}<Text path={at("subdir")} nullable mono label={t("settings.proj.subdir", "Podsložka")} placeholder="—" /></label>

        <Show when={pr().kind === "dev"}>
          <label class="st-sub wide">{L(t("settings.proj.run", "Příkaz dev serveru"), t("settings.proj.run.hint", "Např. npm run dev — spustí se ve složce projektu"))}<Text path={at("run")} nullable mono grow label={t("settings.proj.run", "Příkaz dev serveru")} placeholder="npm run dev" /><FieldErr path={at("run")} /></label>
          <label class="st-sub">{L(t("settings.proj.port", "Port"), t("settings.proj.port.hint", "Obsazený cizím procesem → přidělí se jiný volný"))}<Num path={at("port")} nullable min={1} max={65535} label={t("settings.proj.port", "Port")} placeholder={t("settings.proj.port.auto", "volný")} width="96px" /><FieldErr path={at("port")} /></label>
          <div class="st-sub">
            <span class="st-check"><Toggle path={at("passPort")} label={t("settings.proj.passPort", "Předat port příkazu")} /> {t("settings.proj.passPort", "Předat port příkazu")}<small> (-- --port N)</small></span>
            <span class="st-check"><Toggle path={at("openBrowser")} label={t("settings.proj.openBrowser", "Po naběhnutí otevřít prohlížeč")} /> {t("settings.proj.openBrowser", "Po naběhnutí otevřít prohlížeč")}</span>
          </div>
        </Show>
        <Show when={pr().kind === "app"}>
          <label class="st-sub wide">
            {L(t("settings.proj.exe", "Program (.exe)"), t("settings.proj.exe.hint", "Cesta k .exe (uvnitř projektu se uloží relativně), nebo vyplň jméno nainstalované aplikace"))}
            <PathPick path={at("exe")} kind="exe" nullable label={t("settings.proj.exe", "Program (.exe)")} start={dir()} map={(v) => relativeTo(dir(), v)} placeholder="target\\release\\app.exe" />
            <FieldErr path={at("exe")} />
          </label>
          <label class="st-sub">{L(t("settings.proj.appName", "Nainstalovaná aplikace"), t("settings.proj.appName.hint", "Jméno z Přidat/odebrat programy, když exe nevyplníš"))}<Text path={at("appName")} nullable label={t("settings.proj.appName", "Nainstalovaná aplikace")} placeholder="—" /></label>
          <label class="st-sub">{L(t("settings.proj.aumid", "Aplikace ze Storu (AUMID)"), t("settings.proj.aumid.hint", "Např. Microsoft.WindowsCalculator_8wekyb3d8bbwe!App, když není exe; Ledge ji pak spustí jako ze Start menu"))}<Text path={at("aumid")} nullable mono label={t("settings.proj.aumid", "Aplikace ze Storu (AUMID)")} placeholder="—" /><FieldErr path={at("aumid")} /></label>
          <label class="st-sub">{L(t("settings.proj.prepare", "Příprava před spuštěním"), t("settings.proj.prepare.hint", "Volitelné, např. npm run build"))}<Text path={at("run")} nullable mono label={t("settings.proj.prepare", "Příprava před spuštěním")} placeholder="—" /></label>
          <div class="st-sub wide">{L(t("settings.proj.args", "Argumenty programu"), t("settings.proj.args.hint", "Každý argument zvlášť, Enter přidá"))}<Chips path={at("args")} label={t("settings.proj.args", "Argumenty programu")} placeholder="--flag" /></div>
        </Show>

        <div class="st-sub wide">{L(t("settings.proj.accent", "Barva projektu"))}<Color path={at("accent")} allowEmpty label={t("settings.proj.accent", "Barva projektu")} /><FieldErr path={at("accent")} /></div>
        <label class="st-sub">{L(t("settings.proj.monogram", "Zkratka v logu"), t("settings.proj.monogram.hint", "1–2 znaky, když logo chybí"))}<Text path={at("monogram")} nullable maxLength={2} label={t("settings.proj.monogram", "Zkratka v logu")} placeholder="—" /></label>
        <div class="st-sub">
          {L(t("settings.proj.logo", "Logo"), t("settings.proj.logo.hint", "Prázdné = najde se samo (favicon, ikona exe)"))}
          <div class="st-inline">
            <ProjectLogo project={pr()} size={40} />
            <PathPick path={at("logo")} kind="image" nullable label={t("settings.proj.logo", "Logo")} start={dir()} map={(v) => relativeTo(dir(), v)} placeholder={t("settings.proj.logo.auto", "automaticky")} />
          </div>
        </div>
        <div class="st-sub wide">
          {L(t("settings.proj.links", "Odkazy na dlaždici"), t("settings.proj.links.hint", "Staging, produkce, dokumentace… (jen http/https). Ukážou se jako ikony na dlaždici projektu."))}
          <For each={pr().links ?? []}>
            {(_l, j) => (
              <div class="st-inline">
                <Text path={`${at("links")}.${j()}.label`} label={t("settings.proj.link.label", "Popisek")} placeholder={t("settings.proj.link.label", "Popisek")} maxLength={40} />
                <Text path={`${at("links")}.${j()}.url`} label="URL" placeholder="https://…" mono grow />
                <button class="mini" onClick={() => c.update(at("links"), (l: unknown[]) => l.filter((_, k) => k !== j()))}>{t("settings.ui.remove", "Odebrat")}</button>
                <Show when={linkProblem(pr().links[j()] ?? {})}>{(m) => <div class="st-err" role="alert">{m().field === "label" ? t("settings.proj.link.errLabel", "Doplň popisek odkazu.") : t("settings.proj.link.errUrl", "Adresa musí začínat http:// nebo https://.")}</div>}</Show>
                <FieldErr path={`${at("links")}.${j()}.label`} />
                <FieldErr path={`${at("links")}.${j()}.url`} />
              </div>
            )}
          </For>
          <div class="st-actions"><button class="mini" onClick={() => c.update(at("links"), (l: unknown[] | undefined) => [...(l ?? []), { label: "", url: "" }])}>+ {t("settings.proj.link.add", "Přidat odkaz")}</button></div>
        </div>
      </div>
    </div>
  );
}
