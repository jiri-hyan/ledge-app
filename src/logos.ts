// Loga projektů pro všechna místa v UI (dlaždice, karta Claude, výběr projektu): snapshot nese jen
// `logoKey` (hash obsahu), data URL se z jádra (`project_logo`) vyžádá jednou na klíč a drží se tady.
import { createSignal } from "solid-js";
import { api, type ProjectView } from "./api";

/** Data URL log podle `logoKey` — z jádra jednou na změnu obsahu, ne s každým stavem. */
const [logoUrls, setLogoUrls] = createSignal<Record<string, string>>({});
const logoAsked = new Set<string>();
export function logoSrc(pr: Pick<ProjectView, "id" | "logoKey">): string | undefined {
  const key = pr.logoKey;
  if (!key) return undefined;
  if (!logoAsked.has(key)) {
    logoAsked.add(key);
    api.projectLogo(pr.id).then((url) => url && setLogoUrls((m) => ({ ...m, [key]: url })), () => logoAsked.delete(key));
  }
  return logoUrls()[key];
}
