// Čistá logika panelu Poznámky: zámek Id kategorie. Id, které Ledge vygeneroval při přidání (`kategorie-N`),
// jde jednou přepsat na vlastní; vlastní Id je zamčené, protože změna nepřejmenuje poznámky s `kind:`.

const GENERATED = /^kategorie-\d+$/;

/** Je Id kategorie zamčené (vlastní, už ne vygenerované)? */
export function kindIdLocked(id: string): boolean {
  return !GENERATED.test(id);
}
