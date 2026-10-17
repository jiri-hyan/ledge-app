// Načte všechny anglické slovníky (registerDict) — import v `App.tsx` dřív, než se UI vykreslí.
// Nový slovník = nový import tady; `src/i18n.test.mjs` hlídá, že žádný nechybí.
import "./en-main";
import "./en-cards";
import "./en-sessions";
import "./en-notes";
import "./en-palette";
import "./en-actions";
import "./en-questions";
import "../settings/en";
import "../settings/panels/en";
import "../onboarding/en";
