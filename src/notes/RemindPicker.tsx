// Výběr připomínky místo <input type="datetime-local">: rychlé volby, mini kalendář (česky,
// pondělí první), časy po 15 min + ruční zápis, „Bez připomínky“. Hodnota = lokální
// `YYYY-MM-DDTHH:MM` nebo "" (stejně jako dřív; offset doplní Rust). Logika v remind.ts.

import { createSignal, For, Show } from "solid-js";
import { IconBell } from "../icons";
import { usePopup, swallow, scrollWithin } from "./popup";
import { t } from "../i18n";
import { dnyKr, mesice, TIME_SLOTS, monthGrid, parseHm, parseLocal, pad2, quickPicks, remindText, withDay, withTime } from "./remind";

export default function RemindPicker(p: { value: string; onChange: (v: string) => void; label?: string; ref?: (el: HTMLButtonElement) => void }) {
  const pop = usePopup();
  const now = () => new Date();
  const [month, setMonth] = createSignal<[number, number]>([0, 0]);
  const [typed, setTyped] = createSignal("");
  let trigger!: HTMLButtonElement;

  function openPanel() {
    const d = parseLocal(p.value) ?? now();
    setMonth([d.getFullYear(), d.getMonth()]);
    setTyped(parseLocal(p.value) ? `${d.getHours()}:${pad2(d.getMinutes())}` : "");
    pop.toggle();
  }
  const set = (v: string, close = false) => {
    p.onChange(v);
    if (close) {
      pop.close();
      trigger?.focus();
    }
  };
  const shift = (n: number) => {
    const [y, m] = month();
    const d = new Date(y, m + n, 1);
    setMonth([d.getFullYear(), d.getMonth()]);
  };
  const sel = () => parseLocal(p.value);
  const selHm = () => {
    const d = sel();
    return d ? `${pad2(d.getHours())}:${pad2(d.getMinutes())}` : "";
  };
  function applyTyped() {
    const hm = parseHm(typed());
    if (hm) set(withTime(p.value, hm[0], hm[1], now()));
  }
  function key(e: KeyboardEvent) {
    swallow(e);
    if (e.key === "Escape") {
      e.preventDefault();
      pop.close();
      trigger?.focus();
    }
  }

  return (
    <>
      <span class="pick-field">
        <Show when={p.label}><span class="pick-lbl">{p.label}</span></Show>
        <button
          ref={(el) => {
            trigger = el;
            p.ref?.(el);
          }}
          class="pick-btn"
          classList={{ open: pop.open(), unset: !p.value }}
          data-pop={pop.id}
          onClick={openPanel}
          aria-expanded={pop.open()}
        >
          <IconBell />
          <span class="pick-val">{remindText(p.value, now())}</span>
          <span class="pick-caret" />
        </button>
      </span>
      <Show when={pop.open()}>
        <div class="pick-pop rp" data-pop={pop.id} onKeyDown={key}>
          <div class="rp-quick">
            <For each={quickPicks(now())}>{(q) => <button class="mini" classList={{ on: q.value === p.value }} onClick={() => set(q.value, true)}>{q.label}</button>}</For>
            <button class="mini" classList={{ on: !p.value }} onClick={() => set("", true)}>{t("notes.remind.none", "Bez připomínky")}</button>
          </div>
          <div class="rp-body">
            <div class="rp-cal">
              <div class="rp-head">
                <button class="mini" onClick={() => shift(-1)} aria-label={t("notes.remind.prevMonth", "Předchozí měsíc")}>‹</button>
                <span>{mesice()[month()[1]]} {month()[0]}</span>
                <button class="mini" onClick={() => shift(1)} aria-label={t("notes.remind.nextMonth", "Další měsíc")}>›</button>
              </div>
              <div class="rp-grid">
                <For each={dnyKr()}>{(d) => <span class="rp-dow">{d}</span>}</For>
                <For each={monthGrid(month()[0], month()[1])}>
                  {(d) => {
                    const today = now();
                    const past = new Date(d.getFullYear(), d.getMonth(), d.getDate() + 1).getTime() <= today.getTime();
                    return (
                      <button
                        class="rp-day"
                        classList={{ out: d.getMonth() !== month()[1], today: d.toDateString() === today.toDateString(), sel: sel()?.toDateString() === d.toDateString(), past }}
                        onClick={() => set(withDay(p.value, d, now()))}
                      >
                        {d.getDate()}
                      </button>
                    );
                  }}
                </For>
              </div>
            </div>
            <div class="rp-time">
              <input
                type="text"
                placeholder={t("notes.remind.timePh", "čas, např. 9:30")}
                value={typed()}
                onInput={(e) => setTyped(e.currentTarget.value)}
                onKeyDown={(e) => e.key === "Enter" && (e.preventDefault(), applyTyped())}
                onBlur={applyTyped}
                classList={{ bad: typed().trim() !== "" && !parseHm(typed()) }}
              />
              <div class="rp-slots" ref={(el) => queueMicrotask(() => scrollWithin(el, el.querySelector(".on") ?? el.querySelector("[data-near]"), true))}>
                <For each={TIME_SLOTS}>
                  {(slot) => (
                    <button
                      class="rp-slot"
                      classList={{ on: slot === selHm() }}
                      data-near={!selHm() && slot === "09:00" ? "" : undefined}
                      onClick={() => {
                        const [h, m] = slot.split(":").map(Number);
                        setTyped(`${h}:${pad2(m)}`);
                        set(withTime(p.value, h, m, now()));
                      }}
                    >
                      {slot}
                    </button>
                  )}
                </For>
              </div>
            </div>
          </div>
          <div class="rp-foot">
            <span class="hint">{p.value ? t("notes.remind.will", "Připomenu {when}", { when: remindText(p.value, now()) }) : t("notes.remind.none", "Bez připomínky")}</span>
            <button class="btn pri" onClick={() => { pop.close(); trigger?.focus(); }}>{t("notes.done", "Hotovo")}</button>
          </div>
        </div>
      </Show>
    </>
  );
}
