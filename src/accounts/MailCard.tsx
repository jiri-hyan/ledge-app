// Karta Pošta (účet Microsoft): počet nepřečtených v Doručené poště a 5 nejnovějších zpráv — jen
// předmět, odesílatel a čas (tělo Ledge nečte). Data stahuje Rust (`accounts/mail.rs`) jen s viditelnou
// kartou, nejvýš 1× za 2 minuty; karta jen kreslí. Klik na zprávu / „Otevřít Outlook“ = Outlook na webu.
import { For, Show, type JSX } from "solid-js";
import { api, type MailItem, type Snapshot } from "../api";
import { ago, fmtClock } from "../format";
import { t } from "../i18n";
import { gotoSettingsPanel } from "../palette/goto";
import { sayError } from "../ui/toast";
import { safeLink, unreadBadge } from "./model";
import "./accounts.css";

/** Nastavení → Účty (přihlášení účtu Microsoft). */
export function openAccountsSettings(onSettings: () => void) {
  onSettings();
  gotoSettingsPanel("f-accounts.microsoft");
}

const open = (url: string) => void api.openUrl(url).catch(sayError);

function when(ms: number, now: number): string {
  if (!ms) return "";
  const d = new Date(ms);
  const n = new Date(now);
  return d.toDateString() === n.toDateString() ? fmtClock(d) : t("cards.mail.day", "{d}. {m}.", { d: d.getDate(), m: d.getMonth() + 1 });
}

function Row(p: { it: MailItem; now: number; web: string }) {
  return (
    <button class="row mail-row" classList={{ "mail-unread": p.it.unread }} onClick={() => open(safeLink(p.it.link) ?? p.web)} title={t("cards.mail.openOne", "Otevřít v Outlooku na webu")}>
      <span class="mail-dot" aria-hidden="true" />
      <div class="grow">
        <div class="name">{p.it.subject || t("cards.mail.noSubject", "(bez předmětu)")}</div>
        <div class="sub">{p.it.from}</div>
      </div>
      <span class="mail-time">{when(p.it.receivedMs, p.now)}</span>
    </button>
  );
}

export default function MailCard(p: { s: Snapshot; now: number; onSettings: () => void; menu?: JSX.Element }) {
  const v = () => p.s.personal.mail;
  const status = () => v()?.status ?? "unset";
  const web = () => safeLink(v()?.web) ?? "https://outlook.office.com/mail/";
  return (
    <div class="card" data-anim>
      <h4>
        <span class="ic" style="background:var(--blue)" />
        {t("cards.mail.title", "Pošta")}
        <Show when={unreadBadge(v()?.unread)}>{(n) => <span class="pill b" title={t("cards.mail.unreadTitle", "Nepřečtené v Doručené poště")}>{n()}</span>}</Show>
        {p.menu}
      </h4>
      <Show when={status() !== "unset" && status() !== "off"} fallback={
        <div class="empty mail-empty">
          {status() === "off"
            ? t("cards.mail.off", "Pošta je v Nastavení → Šuplík a karty → Obsah karet → Kalendáře → Pokročilé vypnutá.")
            : t("cards.mail.unset", "Přihlas účet Microsoft (Outlook) a uvidíš tu počet nepřečtených a nejnovější předměty. Ledge čte jen hlavičky zpráv a nic nikam neposílá.")}
          <button class="mini ok" onClick={() => openAccountsSettings(p.onSettings)}>{t("cards.mail.connect", "Nastavit účet…")}</button>
        </div>
      }>
        <Show when={v()?.fetchedMs != null} fallback={
          <div class="empty">
            {status() === "unavailable" ? t("cards.mail.unavailable", "Poštu se nepodařilo načíst.") : t("cards.mail.loading", "Načítám poštu…")}
          </div>
        }>
          <div class="mail-lead">
            {(v()?.unread ?? 0) > 0
              ? t("cards.mail.unread", "Nepřečtené: {n}", { n: v()!.unread! })
              : t("cards.mail.allRead", "Všechno přečtené.")}
          </div>
          <For each={v()?.items ?? []} fallback={<div class="empty">{t("cards.mail.empty", "Doručená pošta je prázdná.")}</div>}>
            {(it) => <Row it={it} now={p.now} web={web()} />}
          </For>
        </Show>
        <Show when={v()?.error}>{(e) => <div class="sub err">{e()}</div>}</Show>
        <div class="mail-foot">
          <button class="mini" onClick={() => open(web())}>{t("cards.mail.openWeb", "Otevřít Outlook")}</button>
          <Show when={status() === "stale" && v()?.fetchedMs != null}><span class="pill a">{t("cards.mail.old", "stará data")}</span></Show>
          <Show when={v()?.fetchedMs}>
            {(f) => <span class="mail-meta" title={v()?.account ?? ""}>{v()?.loading ? t("cards.mail.refreshing", "obnovuji…") : t("cards.mail.fetched", "staženo {ago}", { ago: ago(f(), p.now) })}</span>}
          </Show>
        </div>
      </Show>
    </div>
  );
}
