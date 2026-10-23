//! Malý deterministický generátor (xorshift64) pro vlastnostní a „nepřátelské“ testy bez nové závislosti.
//! Stejné semínko = stejná posloupnost, takže pád jde zopakovat. Jen pro testy (`#[cfg(test)]` v `lib.rs`).

pub(crate) struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    /// Číslo v `0..n` (`n > 0`).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }

    pub fn pick<'a>(&mut self, v: &[&'a str]) -> &'a str {
        v[self.below(v.len())]
    }

    /// 0..=`max` kousků z `v` slepených dohromady.
    pub fn soup(&mut self, v: &[&str], max: usize) -> String {
        let n = self.below(max + 1);
        (0..n).map(|_| self.pick(v)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_sequence_and_bounds_hold() {
        let (mut a, mut b) = (Rng(7), Rng(7));
        assert!((0..100).all(|_| a.next() == b.next()));
        let mut r = Rng(1);
        assert!((0..1000).all(|_| r.below(5) < 5));
        assert!(r.soup(&["a", "b"], 3).len() <= 3);
        assert_eq!(r.soup(&["x"], 0), "");
    }
}
