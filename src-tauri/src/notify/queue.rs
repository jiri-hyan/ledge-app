//! Fronta oznámení Notche (čistá): priorita Čeká > Schůzka > Limit > Časovač > Hotovo, slučování stejných
//! událostí (počet + nejnovější text), „zmeškané“ = nejdůležitější oznámení, které doběhlo bez kliknutí
//! (tečka v proužku do příštího otevření šuplíku).

use super::{Kind, Notice};

/// Co se stalo s novým oznámením.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Push {
    /// nic neběželo → ukaž hned
    Show,
    /// sloučeno s běžícím (stejná událost) → jen přepiš obsah, bez nového rozbalení
    MergedLive,
    /// sloučeno s čekajícím ve frontě
    MergedQueued,
    /// čeká ve frontě
    Queued,
}

/// Nejvýš tolik čekajících (víc = nejméně důležité vypadne; stejné se slučují, takže reálně ≤ 4).
pub const MAX_WAIT: usize = 8;

#[derive(Debug, Default)]
pub struct Queue {
    pub cur: Option<Notice>,
    wait: Vec<Notice>,
    missed: Option<Kind>,
}

fn merge(into: &mut Notice, n: Notice) {
    let count = into.count + n.count.max(1);
    *into = Notice { count, ..n };
}

impl Queue {
    pub fn push(&mut self, n: Notice) -> Push {
        if let Some(c) = self.cur.as_mut().filter(|c| c.kind == n.kind) {
            merge(c, n);
            return Push::MergedLive;
        }
        if let Some(w) = self.wait.iter_mut().find(|w| w.kind == n.kind) {
            merge(w, n);
            return Push::MergedQueued;
        }
        if self.cur.is_none() {
            self.cur = Some(n);
            return Push::Show;
        }
        // za poslední stejně nebo víc důležité (stabilní pořadí příchodu)
        let at = self.wait.iter().position(|w| w.kind.rank() > n.kind.rank()).unwrap_or(self.wait.len());
        self.wait.insert(at, n);
        if self.wait.len() > MAX_WAIT {
            self.wait.pop();
        }
        Push::Queued
    }

    /// Ukázka ze Studia: hned, přes cokoli běžícího (to se nepočítá jako zmeškané).
    pub fn preview(&mut self, n: Notice) {
        self.wait.retain(|w| w.kind != n.kind);
        self.cur = Some(n);
    }

    /// Běžící doběhlo (`acked` = uživatel klikl). Vrátí další k ukázání.
    pub fn finish(&mut self, acked: bool) -> Option<Notice> {
        if let Some(c) = self.cur.take()
            && !acked
            && !c.preview
        {
            self.missed = Some(match self.missed {
                Some(m) if m.rank() <= c.kind.rank() => m,
                _ => c.kind,
            });
        }
        if !self.wait.is_empty() {
            self.cur = Some(self.wait.remove(0));
        }
        self.cur.clone()
    }

    /// Proužek zmizel (hra, šuplík přes něj): nic dalšího se neukáže; neukázané = zmeškané.
    pub fn drop_all(&mut self) {
        let worst = self.cur.iter().chain(self.wait.iter()).filter(|n| !n.preview).map(|n| n.kind).min_by_key(|k| k.rank());
        if let Some(k) = worst {
            self.missed = Some(match self.missed {
                Some(m) if m.rank() <= k.rank() => m,
                _ => k,
            });
        }
        self.cur = None;
        self.wait.clear();
    }

    pub fn missed(&self) -> Option<Kind> {
        self.missed
    }
    pub fn clear_missed(&mut self) {
        self.missed = None;
    }
    #[cfg(test)]
    pub fn waiting(&self) -> usize {
        self.wait.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(kind: Kind, title: &str) -> Notice {
        Notice { kind, title: title.into(), count: 1, ..Notice::default() }
    }

    #[test]
    fn priority_and_merge_table() {
        let mut q = Queue::default();
        assert_eq!(q.push(n(Kind::Done, "a")), Push::Show);
        assert_eq!(q.push(n(Kind::Timer, "t")), Push::Queued);
        assert_eq!(q.push(n(Kind::Done, "b")), Push::MergedLive);
        assert_eq!(q.cur.as_ref().map(|c| (c.title.as_str(), c.count)), Some(("b", 2)));
        assert_eq!(q.push(n(Kind::Meeting, "m")), Push::Queued);
        assert_eq!(q.push(n(Kind::Waiting, "w1")), Push::Queued);
        assert_eq!(q.push(n(Kind::Waiting, "w2")), Push::MergedQueued);
        assert_eq!(q.push(n(Kind::Limit, "l")), Push::Queued);
        // pořadí: Čeká > Schůzka > Limit > Časovač
        let order: Vec<(Kind, u32)> = std::iter::from_fn(|| q.finish(true).map(|c| (c.kind, c.count))).collect();
        assert_eq!(order, [(Kind::Waiting, 2), (Kind::Meeting, 1), (Kind::Limit, 1), (Kind::Timer, 1)]);
        assert_eq!(q.missed(), None, "všechno odkliknuté");
    }

    #[test]
    fn missed_keeps_most_important() {
        let mut q = Queue::default();
        q.push(n(Kind::Done, "d"));
        q.push(n(Kind::Waiting, "w"));
        q.finish(false);
        assert_eq!(q.missed(), Some(Kind::Done));
        q.finish(false);
        assert_eq!(q.missed(), Some(Kind::Waiting));
        q.push(n(Kind::Timer, "t"));
        q.finish(false);
        assert_eq!(q.missed(), Some(Kind::Waiting), "méně důležité nepřebije");
        q.clear_missed();
        assert_eq!(q.missed(), None);
    }

    #[test]
    fn preview_jumps_in_and_is_never_missed() {
        let mut q = Queue::default();
        q.push(n(Kind::Done, "d"));
        q.push(n(Kind::Timer, "t"));
        q.preview(Notice { preview: true, ..n(Kind::Timer, "ukázka") });
        assert_eq!(q.cur.as_ref().map(|c| c.title.as_str()), Some("ukázka"));
        assert_eq!(q.waiting(), 0, "stejná událost z fronty pryč");
        assert_eq!(q.finish(false), None);
        assert_eq!(q.missed(), None);
    }

    #[test]
    fn drop_all_marks_missed_and_caps_queue() {
        let mut q = Queue::default();
        q.push(n(Kind::Done, "d"));
        q.push(n(Kind::Meeting, "m"));
        q.drop_all();
        assert!(q.cur.is_none() && q.waiting() == 0);
        assert_eq!(q.missed(), Some(Kind::Meeting));
        // strop fronty: stejné se slučují, takže ani stovka událostí frontu nenafoukne
        let mut q = Queue::default();
        for i in 0..100 {
            q.push(n(Kind::ALL[i % 5], "x"));
        }
        assert!(q.waiting() <= 4);
    }
}
