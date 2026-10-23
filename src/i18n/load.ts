// Líné načtení anglických slovníků: registruje v `i18n.ts` loader, který při prvním `setLang("en")` (nebo
// `loadDict("en")` z lazy palety) stáhne chunk `dicts.ts`. První import `App.tsx` — dřív než cokoli volá `setLang`.
// Čeština je zdroj pravdy v kódu, takže se při jazyce cs nenačítá žádný slovník (paměť WebView2).
import { setDictLoader } from "../i18n";

setDictLoader(() => import("./dicts"));
