//! Čistá geometrie tapety: obdélníky, relativní poloha dítěte vůči rodiči (záporné souřadnice levého
//! monitoru) a hledání volného místa na ploše pro živou zkoušku (`uncovered_rects` / `pick_probe` ze spike
//! kola 33). Bez Win32, tabulkové testy dole.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct R {
    pub l: i32,
    pub t: i32,
    pub r: i32,
    pub b: i32,
}

impl R {
    pub const fn new(l: i32, t: i32, r: i32, b: i32) -> R {
        R { l, t, r, b }
    }
    pub fn w(&self) -> i32 {
        self.r - self.l
    }
    pub fn h(&self) -> i32 {
        self.b - self.t
    }
    #[allow(dead_code)] // jen testy a `intersect`; sonda volného místa (E3) ho zapojí
    pub fn is_empty(&self) -> bool {
        self.w() <= 0 || self.h() <= 0
    }
    #[allow(dead_code)] // háček pro výběr sondy (E3), zatím jen testy
    pub fn area(&self) -> i64 {
        if self.is_empty() { 0 } else { self.w() as i64 * self.h() as i64 }
    }
    #[allow(dead_code)] // háček pro výběr sondy (E3), zatím jen testy
    pub fn intersect(&self, o: &R) -> Option<R> {
        let x = R::new(self.l.max(o.l), self.t.max(o.t), self.r.min(o.r), self.b.min(o.b));
        if x.is_empty() { None } else { Some(x) }
    }
}

/// Poloha okna-dítěte: `monitor` (obrazovkové souřadnice) vůči levému hornímu rohu klientské oblasti rodiče
/// (`origin`, také obrazovkově). Progman leží přes celou virtuální plochu, takže primární monitor vpravo od
/// levého má kladnou polohu; monitor vlevo od počátku rodiče dá zápornou.
pub fn rel_rect(monitor: R, origin: (i32, i32)) -> R {
    R::new(monitor.l - origin.0, monitor.t - origin.1, monitor.r - origin.0, monitor.b - origin.1)
}

/// `a` minus `cut` → až 4 kusy (nahoře, dole, vlevo, vpravo od průniku).
#[allow(dead_code)] // háček pro sondu volného místa (E3), zatím jen testy
pub fn subtract(a: R, cut: R) -> Vec<R> {
    let Some(x) = a.intersect(&cut) else { return vec![a] };
    let mut v = vec![];
    if x.t > a.t {
        v.push(R::new(a.l, a.t, a.r, x.t));
    }
    if x.b < a.b {
        v.push(R::new(a.l, x.b, a.r, a.b));
    }
    if x.l > a.l {
        v.push(R::new(a.l, x.t, x.l, x.b));
    }
    if x.r < a.r {
        v.push(R::new(x.r, x.t, a.r, x.b));
    }
    v
}

/// Části `areas` (pracovní plochy monitorů bez hlavního panelu), které nezakrývá žádný z `covers`.
#[allow(dead_code)] // háček pro sondu volného místa (E3), zatím jen testy
pub fn uncovered_rects(areas: &[R], covers: &[R]) -> Vec<R> {
    let mut cur: Vec<R> = areas.iter().copied().filter(|r| !r.is_empty()).collect();
    for c in covers {
        cur = cur.into_iter().flat_map(|a| subtract(a, *c)).collect();
    }
    cur
}

/// Okno sondy: největší volný obdélník, který pojme min×min; velikost ≤ max, s odstupem `pad` od okrajů.
#[allow(dead_code)] // háček pro sondu volného místa (E3), zatím jen testy
pub fn pick_probe(free: &[R], min: i32, max: i32, pad: i32) -> Option<R> {
    let mut best: Option<R> = None;
    for f in free {
        let inner = R::new(f.l + pad, f.t + pad, f.r - pad, f.b - pad);
        if inner.w() < min || inner.h() < min {
            continue;
        }
        if best.is_none_or(|b| inner.area() > b.area()) {
            best = Some(inner);
        }
    }
    best.map(|b| {
        let s = b.w().min(b.h()).min(max);
        R::new(b.l, b.t, b.l + s, b.t + s)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rel_rect_table() {
        // (monitor, počátek rodiče) → relativní obdélník
        let cases = [
            // Progman přes 2 monitory 3840 px, levý je záporný: primární je vpravo od počátku
            (R::new(0, 0, 3840, 2160), (-3840, 0), R::new(3840, 0, 7680, 2160)),
            (R::new(-3840, 0, 0, 2160), (-3840, 0), R::new(0, 0, 3840, 2160)),
            // jediný monitor, rodič v počátku
            (R::new(0, 0, 1920, 1080), (0, 0), R::new(0, 0, 1920, 1080)),
            // monitor nad primárním (záporné y)
            (R::new(0, -1080, 1920, 0), (0, -1080), R::new(0, 0, 1920, 1080)),
            // rodič vpravo od monitoru → záporná relativní poloha
            (R::new(0, 0, 100, 100), (50, 20), R::new(-50, -20, 50, 80)),
        ];
        for (mon, origin, want) in cases {
            let got = rel_rect(mon, origin);
            assert_eq!(got, want, "{mon:?} origin {origin:?}");
            assert_eq!((got.w(), got.h()), (mon.w(), mon.h()), "velikost se nesmí změnit");
        }
    }

    #[test]
    fn subtract_cases() {
        let a = R::new(0, 0, 10, 10);
        assert_eq!(subtract(a, R::new(20, 20, 30, 30)), vec![a]);
        assert!(subtract(a, R::new(-5, -5, 15, 15)).is_empty());
        let p = subtract(a, R::new(3, 3, 7, 7));
        assert_eq!(p.len(), 4);
        assert_eq!(p.iter().map(|r| r.area()).sum::<i64>(), 100 - 16);
        assert_eq!(subtract(a, R::new(0, 0, 10, 4)), vec![R::new(0, 4, 10, 10)]);
    }

    #[test]
    fn uncovered_two_monitors() {
        let mons = [R::new(-2560, 0, 0, 1400), R::new(0, 0, 2560, 1400)];
        // jedno okno přes pravý monitor, druhé nechává pruh 100 px vpravo na levém
        let w = [R::new(0, 0, 2560, 1400), R::new(-2560, 0, -100, 1400)];
        let f = uncovered_rects(&mons, &w);
        assert_eq!(f, vec![R::new(-100, 0, 0, 1400)]);
        assert_eq!(pick_probe(&f, 48, 64, 4), Some(R::new(-96, 4, -32, 68)));
    }

    #[test]
    fn fully_covered_is_empty() {
        let f = uncovered_rects(&[R::new(0, 0, 100, 100)], &[R::new(0, 0, 60, 100), R::new(50, 0, 100, 100)]);
        assert!(f.is_empty());
        assert_eq!(pick_probe(&f, 48, 64, 0), None);
    }

    #[test]
    fn pick_skips_small_and_picks_largest() {
        let f = [R::new(0, 0, 40, 500), R::new(100, 0, 300, 200)];
        assert_eq!(pick_probe(&f, 48, 64, 0), Some(R::new(100, 0, 164, 64)));
        assert_eq!(pick_probe(&f, 48, 64, 60), Some(R::new(160, 60, 224, 124)));
    }
}
