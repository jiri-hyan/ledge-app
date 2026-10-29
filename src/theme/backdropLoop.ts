// Smyčka živého pozadí (kolo 25) bez DOM: `createLoop` volá `frame(t, dt)` nejvýš `fps`× za vteřinu a jen mezi
// `start()` a `stop()`. Zastavená smyčka NEMÁ žádnou naplánovanou práci (zruší se čekající rAF), tedy zavřený
// šuplík = nula CPU. Plánovač (`request` / `cancel` / `now`) se vkládá, aby šlo zastavení otestovat bez prohlížeče
// (`backdrop.test.mjs`). Prodleva delší než `MAX_DT` (okno bylo skryté) se do `dt` nezapočítá.
export const MAX_DT = 0.1;

export type LoopDeps = {
  now: () => number;
  request: (cb: () => void) => number;
  cancel: (h: number) => void;
  frame: (t: number, dt: number) => void;
};

export function createLoop(deps: LoopDeps, fps: number) {
  const minMs = 1000 / Math.min(30, Math.max(1, fps));
  let handle = 0;
  let on = false;
  let t0 = 0;
  let last = 0;
  const tick = () => {
    handle = 0;
    if (!on) return;
    const now = deps.now();
    if (now - last >= minMs - 1) {
      const dt = Math.min(MAX_DT, (now - last) / 1000);
      last = now;
      deps.frame((now - t0) / 1000, dt);
    }
    if (on) handle = deps.request(tick);
  };
  return {
    start() {
      if (on) return;
      on = true;
      t0 = last = deps.now();
      handle = deps.request(tick);
    },
    stop() {
      on = false;
      if (handle) deps.cancel(handle);
      handle = 0;
    },
    running: () => on,
    /** je něco naplánované (v klidu musí být false) */
    pending: () => handle !== 0,
  };
}
