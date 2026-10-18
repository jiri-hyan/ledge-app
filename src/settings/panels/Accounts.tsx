// Panel „Účty“: přihlášení účtů Microsoft a Google (OAuth) pro kalendář, poštu a další služby.
// Obsah skládá `accounts/AccountsSettings.tsx`; co z účtů Ledge čte a proč si registraci aplikace
// přináší uživatel sám, vysvětluje panel i `docs/ucty.md`.
import { t } from "../../i18n";
import { PanelHead } from "../fields/Row";
import type { PanelProps } from "../types";
import AccountsSettings from "../../accounts/AccountsSettings";

export default function AccountsPanel(_p: PanelProps) {
  return (
    <>
      <PanelHead title={t("settings.nav.ucty", "Účty")} desc={t("settings.accounts.desc", "Microsoft (Outlook) a Google: kalendář v kartě Kalendář, pošta v kartě Pošta. Jen čtení, jen když je karta vidět, nic se nikam neposílá.")} />
      <AccountsSettings />
    </>
  );
}
