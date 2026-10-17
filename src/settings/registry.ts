// Registr panelů Nastavení: pořadí v postranním menu, id (kotva, hledání) a komponenta. Panely, které
// dodávají jiné oblasti, žijí v `panels/*`; shell je jen vykreslí.
import type { Component } from "solid-js";
import type { PanelProps } from "./types";
import OnboardingPanel from "./panels/Onboarding";
import AppearancePanel from "./panels/Appearance";
import ActionsPanel from "./panels/Actions";
import QuestionsPanel from "./panels/Questions";

/** `kw` = klíčová slova pro hledání jako pár [klíč, čeština] (en ve slovníku `settings.nav.<id>.kw`). */
export type PanelEntry = { id: string; key: string; cs: string; kw: readonly [string, string]; component: Component<PanelProps> };

/** Panely z ostatních oblastí kola 3; vlastní panely shellu (projekty, Claude, poznámky…) přidá shell. */
export const EXTERNAL_PANELS: PanelEntry[] = [
  { id: "vitejte", key: "settings.nav.welcome", cs: "Průvodce", kw: ["settings.nav.welcome.kw", "průvodce první spuštění onboarding"], component: OnboardingPanel },
  { id: "vzhled", key: "settings.nav.appearance", cs: "Vzhled a animace", kw: ["settings.nav.appearance.kw", "motiv barva akcent animace průhlednost písmo hustota proužek"], component: AppearancePanel },
  { id: "akce", key: "settings.nav.actions", cs: "Rychlé akce", kw: ["settings.nav.actions.kw", "akce odkaz web aplikace příkaz paleta"], component: ActionsPanel },
  { id: "otazky", key: "settings.nav.questions", cs: "Otázky Clauda", kw: ["settings.nav.questions.kw", "claude otázka odpověď askuserquestion"], component: QuestionsPanel },
];
