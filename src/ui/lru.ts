// Drobné stropy pro cache, které by jinak rostly s každým novým klíčem (data URL ikon / log / obalů):
// `Map` si pamatuje pořadí vložení, takže nejstarší (nejdéle nepoužitý) klíč je vždy první. Čisté, testy `lru.test.mjs`.

/** Přečte a „osvěží“ záznam (přesune na konec = nejnovější). */
export function lruGet<K, V>(m: Map<K, V>, key: K): V | undefined {
  if (!m.has(key)) return undefined;
  const v = m.get(key) as V;
  m.delete(key);
  m.set(key, v);
  return v;
}

/** Vloží záznam a nejstarší nad `max` zahodí; vrátí zahozené klíče. */
export function lruSet<K, V>(m: Map<K, V>, key: K, value: V, max: number): K[] {
  m.delete(key);
  m.set(key, value);
  const dropped: K[] = [];
  for (const k of m.keys()) {
    if (m.size <= max) break;
    m.delete(k);
    dropped.push(k);
  }
  return dropped;
}

/** Jednou na klíč spustí `load`; výsledek (i rozběhnutý příslib) se drží nejvýš `max` klíčů, chyba se nezapamatuje. */
export function memoLru<K, V>(m: Map<K, Promise<V>>, key: K, load: (k: K) => Promise<V>, max: number): Promise<V> {
  const hit = lruGet(m, key);
  if (hit) return hit;
  const p = load(key);
  lruSet(m, key, p, max);
  p.catch(() => m.get(key) === p && m.delete(key));
  return p;
}

/** Objekt klíč → hodnota (jako signál `logoUrls`) zkrátí na `max` nejnovějších klíčů; vrátí odstraněné klíče. */
export function trimRecord<V>(rec: Record<string, V>, max: number): string[] {
  const keys = Object.keys(rec);
  const drop = keys.slice(0, Math.max(0, keys.length - max));
  for (const k of drop) delete rec[k];
  return drop;
}
