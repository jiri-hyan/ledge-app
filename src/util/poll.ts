// Sdílený vzorkovač s počítáním odběratelů (čistá logika bez DOM, běží i v testu s falešnými časovači). Jedna smyčka
// „zavolej `fetch`, výsledek předej `set`, za `everyMs` znovu“ běží, dokud je aspoň jeden odběratel; první odběratel
// ji spustí hned, poslední ji zastaví (žádný osiřelý časovač). Víc odběratelů téhož zdroje = jeden dotaz, ne N.
export interface SharedPoll {
  /** přihlásí odběratele; vrací odhlášení (opakované volání nic neudělá) */
  acquire(): () => void;
  /** kolik odběratelů teď smyčku drží */
  readonly subscribers: number;
}

export function sharedPoll<T>(fetch: () => Promise<T | null | undefined>, set: (v: T) => void, everyMs: number): SharedPoll {
  let refs = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let gen = 0;
  const run = async (g: number) => {
    try {
      const v = await fetch();
      if (g === gen && v != null) set(v);
    } catch {
      /* jeden vzorek navíc nic neřeší — příště */
    }
    if (g === gen) timer = setTimeout(() => void run(g), everyMs);
  };
  return {
    acquire() {
      if (++refs === 1) void run(++gen);
      let done = false;
      return () => {
        if (done) return;
        done = true;
        if (--refs > 0) return;
        gen++;
        if (timer !== undefined) clearTimeout(timer);
        timer = undefined;
      };
    },
    get subscribers() {
      return refs;
    },
  };
}
