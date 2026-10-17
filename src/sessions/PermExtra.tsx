// Doplňky schválení Clauda pod Povolit/Zamítnout (karta Claude `PendingRow` i `Peek`):
// „Vždy povolit“ s přesným pravidlem a cílovým souborem PŘED klikem (cíl v malém rozbalení, výchozí projekt
// lokálně) a „Zamítnout s důvodem…“ (pole se otevře a vezme fokus až po tomto výslovném kliku).
// Jen klik člověka — žádná klávesa pro „Vždy povolit“. Logika v `permission.ts`, pravidlo vybírá Rust.
import { createSignal, For, onCleanup, Show } from "solid-js";
import { api, type DecideExtra, type Pending, type PermDest } from "../api";
import { say, sayError } from "../ui/toast";
import { t } from "../i18n";
import { DEFAULT_DEST, DESTS, REASON_MAX, alwaysExtra, denyExtra, destFile, reasonKey } from "./permission";
import "../sessions.css";

export default function PermExtra(p: { pd: Pending }) {
  const [dest, setDest] = createSignal<PermDest>(DEFAULT_DEST);
  const [picking, setPicking] = createSignal(false);
  /** `null` = pole důvodu zavřené */
  const [why, setWhy] = createSignal<string | null>(null);
  const [took, setTook] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  let box: HTMLTextAreaElement | undefined;
  /** vrátit fokus, jen když ho vzal klik na „s důvodem…“ (a jen jednou) */
  const release = () => {
    if (!took()) return;
    setTook(false);
    api.releaseFocus();
  };
  // žádost vyřízená jinde / vypršela s otevřeným polem → karta zmizí, fokus nesmí zůstat
  onCleanup(release);

  const destName = (d: PermDest) =>
    d === "localSettings" ? t("sessions.perm.destLocal", "tento projekt")
    : d === "userSettings" ? t("sessions.perm.destUser", "všechny projekty")
    : t("sessions.perm.destSession", "jen tato session");
  const destWhere = (d: PermDest) => destFile(d) ?? t("sessions.perm.nowhere", "nikam se nezapíše");

  async function send(allow: boolean, extra: DecideExtra | undefined) {
    if (busy()) return;
    setBusy(true);
    try {
      if (!(await api.decide(p.pd.id, allow, extra))) say(t("sessions.perm.gone", "Žádost už nečeká — vyřídila se jinde nebo vypršela."), true);
    } catch (e) {
      sayError(e);
    } finally {
      setBusy(false);
      release();
    }
  }

  /** Výslovný klik → šuplík dostane klávesnici a fokus jde do pole (jinak Ledge fokus nebere). */
  function openWhy() {
    setWhy("");
    setTook(true);
    api.focusDrawer();
    queueMicrotask(() => box?.focus());
  }
  function closeWhy() {
    setWhy(null);
    release();
  }
  function key(e: KeyboardEvent) {
    const k = reasonKey(e.key, e.shiftKey, why() ?? "");
    if (e.key === "Escape" || (e.key === "Enter" && !e.shiftKey)) e.preventDefault();
    if (e.key === "Escape") e.stopPropagation(); // Esc jinak zavře šuplík
    if (k === "cancel") closeWhy();
    else if (k === "send") void send(false, denyExtra(why() ?? ""));
  }

  return (
    <div class="sess-perm">
      <Show when={p.pd.always}>
        {(a) => (
          <>
            <div class="sess-perm-line">
              <button class="mini" disabled={busy()} onClick={() => send(true, alwaysExtra(dest()))}
                title={t("sessions.perm.alwaysHint", "Povolí teď a Claude Code si pravidlo zapíše — příště se nezeptá")}>
                {t("sessions.perm.always", "Vždy povolit")}
              </button>
              <code class="sess-perm-rule" title={a().fromClaude ? t("sessions.perm.fromClaude", "Návrh pravidla od Clauda") : t("sessions.perm.fromLedge", "Přesně tento příkaz (bez návrhu od Clauda)")}>{a().label}</code>
              <button class="sess-perm-dest" aria-expanded={picking()} onClick={() => setPicking(!picking())}
                title={dest() === "localSettings" ? t("sessions.perm.localHint", "Do .claude/settings.local.json v kořeni projektu session (necommituje se)") : undefined}>
                → {destName(dest())} · {destWhere(dest())} {picking() ? "▴" : "▾"}
              </button>
            </div>
            <Show when={picking()}>
              <div class="sess-perm-line">
                <For each={DESTS}>
                  {(d) => (
                    <button class="sess-perm-opt" classList={{ on: d === dest() }} aria-pressed={d === dest()} onClick={() => { setDest(d); setPicking(false); }}>
                      {destName(d)} <small>{destWhere(d)}</small>
                    </button>
                  )}
                </For>
              </div>
            </Show>
          </>
        )}
      </Show>
      <Show when={why() !== null} fallback={
        <button class="sess-perm-link" disabled={busy()} onClick={openWhy}>{t("sessions.perm.denyWhy", "Zamítnout s důvodem…")}</button>
      }>
        <textarea
          ref={box}
          class="sess-perm-why"
          rows={2}
          maxLength={REASON_MAX}
          placeholder={t("sessions.perm.whyPlaceholder", "Proč ne / co místo toho… (Enter odešle Claudovi, Esc zruší)")}
          value={why() ?? ""}
          disabled={busy()}
          onInput={(e) => setWhy(e.currentTarget.value)}
          onKeyDown={key}
        />
        <div class="sess-perm-line">
          <button class="mini" disabled={busy()} onClick={closeWhy}>{t("sessions.perm.cancel", "Zrušit")}</button>
          <button class="mini" disabled={busy() || !denyExtra(why() ?? "")} onClick={() => send(false, denyExtra(why() ?? ""))}>{t("sessions.perm.denySend", "Zamítnout a poslat důvod")}</button>
          <small class="sess-perm-count">{(why() ?? "").length}/{REASON_MAX}</small>
        </div>
      </Show>
    </div>
  );
}
