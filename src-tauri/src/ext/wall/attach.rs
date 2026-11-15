//! Kam připnout okno tapety. Čisté funkce nad popisem stromu oken (`Tree`): `choose_parent` vybere rodiče,
//! `attach_plan` řekne, co s oknem udělat po startu / restartu Průzkumníka / změně stromu. Sběr stromu z
//! Win32 (`win::snapshot`) je v `wall/mod.rs`.
//!
//! Poznatky spike kola 33 (Win 11 build 26200): Progman má dítě `WorkerW` (po `0x052C`) a `SHELLDLL_DefView`
//! (ikony) nad ním; WebView2 jako dítě `WorkerW` i Progmanu se vykreslí, GDI děti ne. Starší sestavení
//! (23H2) mají `WorkerW` jako sourozence Progmanu na nejvyšší úrovni.

/// Rodič okna tapety.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParentKind {
    /// `WorkerW` jako dítě Progmanu (Win 11 24H2+, ověřeno 96 %)
    WorkerWChild,
    /// top-level `WorkerW` bez `SHELLDLL_DefView` (starší sestavení)
    WorkerWTop,
    /// dítě Progmanu hned pod `SHELLDLL_DefView` (varianta C spike, 96 %); když `WorkerW` chybí
    ProgmanBehindIcons,
}

/// Co o našem okně víme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ours {
    /// okno ještě existuje (Progman zahynul → zahynuly i jeho děti)
    pub alive: bool,
    pub parent: ParentKind,
    /// rodič je ten samý HWND, který hledání vrací teď
    pub same_parent: bool,
}

/// Popis stromu oken (fakta, žádné HWND — kvůli tabulkovým testům).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Tree {
    pub progman: bool,
    pub workerw_child: bool,
    pub workerw_top: bool,
    pub ours: Option<Ours>,
}

/// Co s oknem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// nic, vše sedí
    Reuse,
    /// okno žije, jen změnit rodiče (`SetParent` + nová poloha)
    Reparent(ParentKind),
    /// vytvořit okno i WebView znovu
    Recreate(ParentKind),
    /// není kam připnout (Průzkumník neběží) — čekat na `TaskbarCreated`
    Fallback,
}

/// Preferovaný rodič: `WorkerW` v Progmanu, pak top-level `WorkerW`, nakonec přímo Progman. `None` bez Progmanu.
pub fn choose_parent(t: &Tree) -> Option<ParentKind> {
    if !t.progman {
        return None;
    }
    Some(if t.workerw_child {
        ParentKind::WorkerWChild
    } else if t.workerw_top {
        ParentKind::WorkerWTop
    } else {
        ParentKind::ProgmanBehindIcons
    })
}

/// Plán připnutí.
pub fn attach_plan(t: &Tree) -> Action {
    let Some(want) = choose_parent(t) else { return Action::Fallback };
    match t.ours {
        None => Action::Recreate(want),
        Some(o) if !o.alive => Action::Recreate(want),
        Some(o) if o.parent == want && o.same_parent => Action::Reuse,
        Some(_) => Action::Reparent(want),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ParentKind::*;

    const fn tree(progman: bool, wc: bool, wt: bool, ours: Option<Ours>) -> Tree {
        Tree { progman, workerw_child: wc, workerw_top: wt, ours }
    }
    const fn ours(alive: bool, parent: ParentKind, same: bool) -> Option<Ours> {
        Some(Ours { alive, parent, same_parent: same })
    }

    #[test]
    fn choose_parent_table() {
        let cases = [
            (tree(false, true, true, None), None),
            (tree(true, true, false, None), Some(WorkerWChild)),
            (tree(true, true, true, None), Some(WorkerWChild)),
            (tree(true, false, true, None), Some(WorkerWTop)),
            (tree(true, false, false, None), Some(ProgmanBehindIcons)),
        ];
        for (t, want) in cases {
            assert_eq!(choose_parent(&t), want, "{t:?}");
        }
    }

    #[test]
    fn attach_plan_table() {
        let cases = [
            // Průzkumník neběží
            ("bez Progmanu", tree(false, false, false, None), Action::Fallback),
            ("bez Progmanu, okno žije", tree(false, false, false, ours(true, WorkerWChild, true)), Action::Fallback),
            // první start
            ("první start, WorkerW v Progmanu", tree(true, true, false, None), Action::Recreate(WorkerWChild)),
            ("první start, starý strom", tree(true, false, true, None), Action::Recreate(WorkerWTop)),
            ("první start, bez WorkerW", tree(true, false, false, None), Action::Recreate(ProgmanBehindIcons)),
            // vše sedí (idempotentní re-attach po simulovaném TaskbarCreated)
            ("beze změny", tree(true, true, false, ours(true, WorkerWChild, true)), Action::Reuse),
            // Progman nový po restartu Průzkumníka: okno i rodič zahynuly
            ("Progman nový", tree(true, true, false, ours(false, WorkerWChild, false)), Action::Recreate(WorkerWChild)),
            // WorkerW byl nahrazen jiným, okno žije
            ("jiný HWND rodiče", tree(true, true, false, ours(true, WorkerWChild, false)), Action::Reparent(WorkerWChild)),
            // vznikl lepší rodič (0x052C doběhla pozdě)
            ("lepší rodič", tree(true, true, false, ours(true, ProgmanBehindIcons, true)), Action::Reparent(WorkerWChild)),
            // rodič zmizel, okno žije (přežilo jako sirotek)
            ("rodič zmizel", tree(true, false, false, ours(true, WorkerWChild, false)), Action::Reparent(ProgmanBehindIcons)),
        ];
        for (name, t, want) in cases {
            assert_eq!(attach_plan(&t), want, "{name}");
        }
    }
}
