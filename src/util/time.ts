// Sdílený debounce: zavolání se odloží o `ms` od POSLEDNÍHO volání, každé nové čekání začne znovu. Nahrazuje ruční
// dvojice `clearTimeout` + `setTimeout` (koncept poznámky, rozepsaná úprava, lístek, lišta Zpět). Čistá logika bez
// DOM (běží i v testu přes `node:test` mock timers): `flush()` provede čekající volání hned, `cancel()` ho zahodí.

export interface Debounced<A extends unknown[]> {
  (...args: A): void;
  /** čeká se na zavolání? */
  readonly pending: boolean;
  /** čekající volání provede hned (nic nečeká = nic neudělá) */
  flush(): void;
  /** čekající volání zahodí */
  cancel(): void;
}

export function debounce<A extends unknown[]>(fn: (...args: A) => void, ms: number): Debounced<A> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let last: A | undefined;
  const run = () => {
    timer = undefined;
    const a = last as A;
    last = undefined;
    fn(...a);
  };
  const d = ((...args: A) => {
    last = args;
    if (timer !== undefined) clearTimeout(timer);
    timer = setTimeout(run, ms);
  }) as Debounced<A>;
  Object.defineProperty(d, "pending", { get: () => timer !== undefined });
  d.flush = () => {
    if (timer === undefined) return;
    clearTimeout(timer);
    run();
  };
  d.cancel = () => {
    if (timer !== undefined) clearTimeout(timer);
    timer = undefined;
    last = undefined;
  };
  return d;
}
