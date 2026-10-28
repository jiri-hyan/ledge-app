//! Drobné čisté helpery sdílené modulem napříč aplikací: čas, zkracování textu, procenta.
//! Jen to, co mělo v kódu identické kopie; varianty s jinou sémantikou zůstávají u svých volajících.

use std::time::{SystemTime, UNIX_EPOCH};

/// Aktuální čas, ms od epochy (hodiny před epochou = 0).
pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// Totéž jako `now_ms`, ale `i64` (časy ukládané do configu / snímků jsou podepsané).
pub fn now_ms_i64() -> i64 {
    i64::try_from(now_ms()).unwrap_or(i64::MAX)
}

/// Zkrátí na `max` znaků (ne bajtů — nikdy neřeže uprostřed UTF-8); delší text skončí na `…`
/// (celkem tedy nejvýš `max` znaků).
pub fn cut(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Jeden řádek: bílé znaky (i nové řádky) sloučené na jednu mezeru, pak `cut`.
pub fn clip_line(s: &str, max: usize) -> String {
    cut(&s.split_whitespace().collect::<Vec<_>>().join(" "), max)
}

/// Procenta `n` z `total`, saturující: `n > total` = 100, `total == 0` = 0.
pub fn pct(n: u64, total: u64) -> u64 {
    if total == 0 {
        return 0;
    }
    (u128::from(n.min(total)) * 100 / u128::from(total)) as u64
}

/// „42 %“ z procent jako desetinné číslo (zaokrouhleno, ořezáno na 0–100; NaN = 0 %).
pub fn pct_text(p: f64) -> String {
    let p = if p.is_nan() { 0.0 } else { p };
    format!("{:.0} %", p.round().clamp(0.0, 100.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn now_is_plausible_and_consistent() {
        let (a, b) = (now_ms(), now_ms_i64());
        assert!(a > 1_700_000_000_000 && b > 0 && (b as u64).abs_diff(a) < 5_000);
    }

    #[test]
    fn cut_table() {
        for (s, max, want) in [
            ("abc", 3, "abc"),
            ("abcd", 3, "ab…"),
            ("", 0, ""),
            ("a", 0, "…"),
            ("abcd", 1, "…"),
            ("ěščřž", 5, "ěščřž"),
            ("ěščřžý", 5, "ěščř…"),
            ("日本語テキスト", 4, "日本語…"),
            ("😀😀😀", 2, "😀…"),
        ] {
            assert_eq!(cut(s, max), want, "{s:?} {max}");
        }
    }

    #[test]
    fn clip_line_table() {
        for (s, max, want) in [
            ("  a \n\n b\t c ", 20, "a b c"),
            ("řádek\ndruhý řádek", 8, "řádek d…"),
            ("", 5, ""),
            ("   ", 5, ""),
            ("abcdef", 6, "abcdef"),
        ] {
            assert_eq!(clip_line(s, max), want, "{s:?} {max}");
        }
    }

    #[test]
    fn pct_saturates() {
        for (n, t, want) in [(0, 0, 0), (5, 0, 0), (0, 10, 0), (5, 10, 50), (10, 10, 100), (30, 10, 100), (1, 3, 33), (u64::MAX, u64::MAX, 100)] {
            assert_eq!(pct(n, t), want, "{n}/{t}");
        }
    }

    #[test]
    fn pct_text_table() {
        for (p, want) in [(0.0, "0 %"), (41.6, "42 %"), (99.5, "100 %"), (250.0, "100 %"), (-3.0, "0 %"), (f64::NAN, "0 %"), (f64::INFINITY, "100 %")] {
            assert_eq!(pct_text(p), want, "{p}");
        }
    }
}
