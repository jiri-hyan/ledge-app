// Nastavení → Osobní → Kalendáře → Pokročilé (skládá `settings/panels/Personal.tsx`): přidat účet (dlaždice Microsoft / Google), stav (přihlášen jako…, co Ledge smí
// číst), Odhlásit a průvodce vlastní registrací aplikace (Client ID). Ledge nemá vlastní ověřenou
// registraci — výchozí Client ID je prázdné a panel řekne proč. Přihlášení samo běží v systémovém
// prohlížeči; refresh token a client secret jdou rovnou do Správce pověření Windows (Rust), do configu
// jen Client ID. Volby (co číst, kalendáře Google) mění draft — uloží je lišta Nastavení.
import { createSignal, For, onMount, Show, type JSX } from "solid-js";
import { t } from "../i18n";
import { hit, useSettings } from "../settings/ctx";
import { Confirm } from "../ui/Confirm";
import { Text, Toggle } from "../settings/fields/controls";
import { LiveBadge, Row, Section } from "../settings/fields/Row";
import { fieldHint, fieldLabel } from "../settings/labels";
import { say, sayError } from "../ui/toast";
import { accountsApi, type AccountView, type GoogleCalendar } from "./api";
import { calendarOn, canConnect, grants, toggleCalendar, type Grant, type ProviderId } from "./model";
import "./accounts.css";

const LOGO: Record<ProviderId, JSX.Element> = {
  microsoft: (
    <svg class="acc-logo" viewBox="0 0 20 20" aria-hidden="true">
      <rect x="1" y="1" width="8.5" height="8.5" fill="#f25022" />
      <rect x="10.5" y="1" width="8.5" height="8.5" fill="#7fba00" />
      <rect x="1" y="10.5" width="8.5" height="8.5" fill="#00a4ef" />
      <rect x="10.5" y="10.5" width="8.5" height="8.5" fill="#ffb900" />
    </svg>
  ),
  google: (
    <svg class="acc-logo" viewBox="0 0 20 20" aria-hidden="true">
      <circle cx="10" cy="10" r="8" fill="none" stroke="#4285f4" stroke-width="3" />
      <path d="M10 10h8" stroke="#34a853" stroke-width="3" />
    </svg>
  ),
};

const grantText = (g: Grant): string =>
  ({
    profile: t("settings.accounts.grant.profile", "jméno a e-mail účtu"),
    calendar: t("settings.accounts.grant.calendar", "kalendář — jen čtení událostí"),
    mail: t("settings.accounts.grant.mail", "pošta — jen počet nepřečtených a hlavičky zpráv (předmět, odesílatel), ne obsah"),
    offline: t("settings.accounts.grant.offline", "zůstat přihlášený (obnova bez nového přihlášení)"),
  })[g];

function MsGuide() {
  return (
    <details class="acc-guide">
      <summary>{t("settings.accounts.ms.guide", "Jak získat Client ID (Azure Portal, asi 5 minut)")}</summary>
      <ol>
        <li>{t("settings.accounts.ms.g1", "Otevři portal.azure.com a přihlas se (stačí osobní účet Microsoft). Najdi „Microsoft Entra ID“ → „App registrations“ (Registrace aplikací) → „New registration“.")}</li>
        <li>{t("settings.accounts.ms.g2", "Name: třeba „Ledge (moje)“. Supported account types: „Accounts in any organizational directory and personal Microsoft accounts“ — pak půjde osobní i pracovní účet.")}</li>
        <li>{t("settings.accounts.ms.g3", "Redirect URI: platforma „Public client/native (mobile & desktop)“, adresa http://localhost (bez portu — Ledge si port volí sám). Klikni Register.")}</li>
        <li>{t("settings.accounts.ms.g4", "V přehledu (Overview) zkopíruj „Application (client) ID“ a vlož ho výš. Žádný client secret nevytvářej — Ledge je veřejný klient s PKCE.")}</li>
        <li>{t("settings.accounts.ms.g5", "Authentication → „Allow public client flows“ = Yes → Save. Oprávnění (API permissions) přidávat nemusíš — Ledge si o ně řekne při přihlášení: User.Read, Calendars.Read, Mail.ReadBasic.")}</li>
        <li>{t("settings.accounts.ms.g6", "Pracovní nebo školní účet: správce organizace může aplikace mimo firmu zakázat — pak Microsoft napíše, že je potřeba schválení správcem. Požádej správce, nebo použij osobní účet.")}</li>
      </ol>
    </details>
  );
}

function GoogleGuide() {
  return (
    <details class="acc-guide">
      <summary>{t("settings.accounts.g.guide", "Jak získat Client ID (Google Cloud Console, asi 10 minut)")}</summary>
      <ol>
        <li>{t("settings.accounts.g.g1", "Otevři console.cloud.google.com, vlevo nahoře vytvoř nový projekt (třeba „Ledge“).")}</li>
        <li>{t("settings.accounts.g.g2", "APIs & Services → Library → vyhledej „Google Calendar API“ → Enable.")}</li>
        <li>{t("settings.accounts.g.g3", "Google Auth Platform (OAuth consent screen) → Get started: název aplikace, tvůj e-mail, Audience = External. V Data access přidej scope …/auth/calendar.readonly.")}</li>
        <li>{t("settings.accounts.g.g4", "Audience → Test users → Add users → přidej svůj Gmail. Neověřená aplikace smí jen testovací uživatele (nejvýš 100) a Google při přihlášení ukáže varování „Google tuto aplikaci neověřil“.")}</li>
        <li>{t("settings.accounts.g.g5", "Clients → Create client → typ „Desktop app“ → Create. Zkopíruj Client ID a Client secret výš (u desktopové aplikace secret tajný není, Ledge ho i tak drží jen ve Správci pověření Windows). Adresu přesměrování nezadáváš.")}</li>
        <li>{t("settings.accounts.g.g6", "Pozor: v režimu „Testing“ Google přihlášení po 7 dnech zneplatní. Až to funguje, přepni v Audience „Publishing status“ na „In production“ (bez ověření, jen pro sebe) — varování zůstane, přihlášení už nevyprší.")}</li>
      </ol>
    </details>
  );
}

function Provider(p: { id: ProviderId; view?: AccountView; open: boolean; onChanged: () => void }) {
  const c = useSettings();
  const base = `accounts.${p.id}`;
  const [secret, setSecret] = createSignal("");
  const [waiting, setWaiting] = createSignal(false);
  const [ask, setAsk] = createSignal(false);
  const [cals, setCals] = createSignal<GoogleCalendar[] | null>(null);
  const clientId = (): string => c.get(`${base}.clientId`) ?? "";
  const connected = () => !!p.view?.connected && !p.view.expired;
  const ready = () => canConnect(p.id, clientId(), secret());
  const label = p.id === "microsoft" ? t("settings.accounts.ms.name", "Microsoft — Outlook, Microsoft 365") : t("settings.accounts.g.name", "Google — Kalendář Google");

  async function connect() {
    setWaiting(true);
    try {
      const v = await accountsApi.connect(p.id, clientId(), p.id === "google" ? secret() : undefined, p.id === "microsoft" ? c.get(`${base}.tenant`) ?? "common" : undefined);
      if (v) say(t("settings.accounts.done", "Přihlášeno jako {a}. Ulož Nastavení, ať si Ledge Client ID pamatuje.", { a: v.account }));
      setSecret("");
    } catch (e) {
      sayError(e);
    } finally {
      setWaiting(false);
      p.onChanged();
    }
  }
  async function disconnect() {
    setAsk(false);
    try {
      await accountsApi.disconnect(p.id);
      say(t("settings.accounts.signedOut", "Odhlášeno — přihlášení je smazané ze Správce pověření."));
    } catch (e) {
      sayError(e);
    } finally {
      p.onChanged();
    }
  }
  async function loadCals() {
    try {
      setCals((await accountsApi.googleCalendars()) ?? []);
    } catch (e) {
      sayError(e);
    }
  }
  const selected = (): string[] => c.get("accounts.google.calendars") ?? [];

  return (
    <Section id={`accounts.${p.id}`} title={label} desc={fieldHint(base)}>
      <Show when={p.open || connected() || hit()?.startsWith(base)}>
        <Show when={connected()} fallback={
          <>
            <Show when={p.view?.expired}><div class="acc-state acc-state-bad">{p.view?.error ?? t("settings.accounts.expired", "Přihlášení vypršelo — přihlas se znovu.")}</div></Show>
            <Row id={`accounts.${p.id}.clientId`} stack>
              <Text path={`${base}.clientId`} label={fieldLabel(`${base}.clientId`)} placeholder={p.id === "microsoft" ? "00000000-0000-0000-0000-000000000000" : "…apps.googleusercontent.com"} mono grow maxLength={160} />
            </Row>
            <Show when={p.id === "google"}>
              <Row id="accounts.google.secret" label={t("settings.accounts.g.secret", "Client secret")} hint={t("settings.accounts.g.secret.hint", "Z Google Cloud Console (klient Desktop app). Uloží se jen do Správce pověření Windows, ne do configu.")} stack>
                <input type="password" class="acc-secret" autocomplete="off" spellcheck={false} aria-label={t("settings.accounts.g.secret", "Client secret")} placeholder="GOCSPX-…" value={secret()} onInput={(e) => setSecret(e.currentTarget.value)} />
              </Row>
            </Show>
            <Show when={p.id === "microsoft"}>
              <Row id="accounts.microsoft.tenant" label={t("settings.accounts.ms.tenant", "Kdo se smí přihlásit")} hint={t("settings.accounts.ms.tenant.hint", "common = osobní i pracovní účty; organizations = jen pracovní; consumers = jen osobní; nebo ID tvé organizace.")}>
                <Text path="accounts.microsoft.tenant" label={t("settings.accounts.ms.tenant", "Kdo se smí přihlásit")} placeholder="common" mono maxLength={64} />
              </Row>
            </Show>
            <p class="acc-why">
              {p.id === "microsoft"
                ? t("settings.accounts.ms.consent", "Po přihlášení Ledge čte jméno účtu, události kalendáře a v poště jen počet nepřečtených a předměty 5 nejnovějších zpráv. Nic neposílá dál — data jsou jen pro zobrazení v šuplíku.")
                : t("settings.accounts.g.consent", "Po přihlášení Ledge čte jen události tvého kalendáře Google. Poštu Gmail ne — Google pro ni vyžaduje placený bezpečnostní audit. Nic neposílá dál.")}
            </p>
            <div class="acc-bar">
              <Show when={!waiting()} fallback={
                <>
                  <span class="acc-wait">{t("settings.accounts.waiting", "Čekám na přihlášení v prohlížeči… (nejvýš 5 minut)")}</span>
                  <button class="mini" onClick={() => void accountsApi.cancel(p.id)}>{t("settings.accounts.cancel", "Zrušit")}</button>
                </>
              }>
                <LiveBadge />
                <button class="mini ok" disabled={!ready()} onClick={() => void connect()} title={ready() ? "" : t("settings.accounts.needId", "Nejdřív vyplň údaje z registrace aplikace (průvodce níž).")}>
                  {p.view?.expired ? t("settings.accounts.reconnect", "Přihlásit znovu v prohlížeči") : t("settings.accounts.connect", "Přihlásit v prohlížeči")}
                </button>
              </Show>
            </div>
            <Show when={p.view?.error && !p.view?.expired}><div class="acc-state acc-state-bad">{p.view!.error}</div></Show>
            {p.id === "microsoft" ? <MsGuide /> : <GoogleGuide />}
          </>
        }>
          <div class="acc-state acc-state-ok">
            {p.view?.name ? t("settings.accounts.as", "Přihlášen jako {a} ({n})", { a: p.view!.account, n: p.view!.name }) : t("settings.accounts.asOnly", "Přihlášen jako {a}", { a: p.view?.account ?? "" })}
          </div>
          <div class="acc-state">{t("settings.accounts.reads", "Ledge smí číst:")}</div>
          <ul class="acc-reads">
            <For each={grants(p.view?.scopes ?? [])}>{(g) => <li>{grantText(g)}</li>}</For>
          </ul>
          <p class="acc-why">{t("settings.accounts.noSend", "Nic z toho Ledge nikam neposílá a nic nemění. Stahuje jen, když je karta vidět.")}</p>
          <Row id={`accounts.${p.id}.calendar`}><Toggle path={`${base}.calendar`} label={fieldLabel(`${base}.calendar`)} /></Row>
          <Show when={p.id === "microsoft"}>
            <Row id="accounts.microsoft.mail"><Toggle path="accounts.microsoft.mail" label={fieldLabel("accounts.microsoft.mail")} /></Row>
          </Show>
          <Show when={p.id === "google" && c.get("accounts.google.calendar")}>
            <Row id="accounts.google.calendars" stack>
              <Show when={cals()} fallback={<button class="mini" onClick={() => void loadCals()}>{t("settings.accounts.g.pick", "Vybrat kalendáře…")}</button>}>
                {(list) => (
                  <div class="acc-cals">
                    <For each={list()}>
                      {(k) => {
                        // prázdný výběr = hlavní kalendář; poslední zaškrtnutý nejde odškrtnout (vypnutí = přepínač výš)
                        const effective = () => (selected().length ? selected() : list().filter((x) => x.primary).map((x) => x.id));
                        const on = () => calendarOn(selected(), k.id, k.primary);
                        return (
                        <label class="acc-cal">
                          <input type="checkbox" checked={on()} disabled={on() && effective().length === 1} onChange={() => c.set("accounts.google.calendars", toggleCalendar(effective(), k.id))} />
                          <span class="acc-cal-dot" style={{ background: k.color ?? "var(--green)" }} />
                          {k.name}
                        </label>
                        );
                      }}
                    </For>
                  </div>
                )}
              </Show>
            </Row>
          </Show>
          <div class="acc-bar">
            <LiveBadge />
            <button class="mini" onClick={() => setAsk(true)}>{t("settings.accounts.signOut", "Odhlásit")}</button>
          </div>
          <Confirm
            when={ask()}
            text={t("settings.accounts.signOutQ", "Odhlásit účet {a}?", { a: p.view?.account ?? "" })}
            detail={p.id === "microsoft"
              ? t("settings.accounts.ms.signOutDetail", "Ledge smaže přihlášení ze Správce pověření. Microsoft odvolání pro tenhle typ aplikace nenabízí — přístup můžeš odebrat i na account.microsoft.com → Soukromí → Aplikace a služby.")
              : t("settings.accounts.g.signOutDetail", "Ledge přihlášení u Googlu odvolá a smaže ze Správce pověření.")}
            yes={t("settings.accounts.signOut", "Odhlásit")}
            danger
            onYes={() => void disconnect()}
            onNo={() => setAsk(false)}
          />
        </Show>
      </Show>
    </Section>
  );
}

export default function AccountsSettings() {
  const [views, setViews] = createSignal<AccountView[]>([]);
  const [open, setOpen] = createSignal<ProviderId | null>(null);
  const reload = () => accountsApi.status().then((v) => v && setViews(v), sayError);
  onMount(reload);
  const view = (id: ProviderId) => views().find((v) => v.provider === id);
  const state = (id: ProviderId): string => {
    const v = view(id);
    if (v?.connecting) return t("settings.accounts.st.connecting", "přihlašuje se…");
    if (v?.expired) return t("settings.accounts.st.expired", "přihlášení vypršelo");
    if (v?.connected) return v.account;
    return t("settings.accounts.st.none", "nepřipojeno");
  };
  const tile = (id: ProviderId, name: string, what: string) => (
    <button class="acc-tile" classList={{ "acc-tile-on": open() === id || !!view(id)?.connected }} onClick={() => setOpen(open() === id ? null : id)} aria-expanded={open() === id}>
      <span class="acc-tile-h">{LOGO[id]}{name}</span>
      <small>{what}</small>
      <span class="acc-state" classList={{ "acc-state-ok": !!view(id)?.connected && !view(id)?.expired }}>{state(id)}</span>
    </button>
  );
  return (
    <>
      <Section title={t("settings.accounts.add", "Přidat účet")} desc={t("settings.accounts.addDesc", "Vyber účet. Integrace jsou vypnuté, dokud se nepřihlásíš — bez přihlášení Ledge na Microsoft ani Google nic neposílá.")}>
        <div class="acc-tiles">
          {tile("microsoft", "Microsoft", t("settings.accounts.ms.what", "Outlook kalendář a pošta (počet nepřečtených)"))}
          {tile("google", "Google", t("settings.accounts.g.what", "Kalendář Google"))}
        </div>
        <p class="acc-why">{t("settings.accounts.why", "Ledge je open-source appka bez vlastní ověřené registrace u Microsoftu a Googlu, proto si ji vytvoříš sám (zdarma, krok za krokem v průvodci). Tvoje Client ID patří jen tobě; přihlášení zůstává ve Správci pověření Windows.")}</p>
      </Section>
      <Provider id="microsoft" view={view("microsoft")} open={open() === "microsoft"} onChanged={reload} />
      <Provider id="google" view={view("google")} open={open() === "google"} onChanged={reload} />
    </>
  );
}
