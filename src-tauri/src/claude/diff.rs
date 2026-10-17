//! Řádkový diff pro náhled v kartě schválení (`preview.rs`). Čisté, bez I/O.
//!
//! - `ops`: Myers (O((N+M)·D), paměť O(D²)) nad řádky po oříznutí společného začátku a konce. Rozdíl
//!   větší než `MAX_D` editací se nepočítá — prostředek se ukáže jako „smazat vše, přidat vše“ (`rough`),
//!   aby velký přepis nesežral CPU ani paměť.
//! - `render`: z operací udělá řádky karty — změny + `CONTEXT` řádků kolem, vynechané úseky jako `Gap`,
//!   nejvýš `max_lines` řádků (zbytek spočítá do `cut`). Počty `added`/`removed` jsou vždy celé.
//!
//! Mapa: `docs/codemap/sessions.md` → Schvalování.

use serde::Serialize;

/// Řádků kontextu kolem změny.
pub const CONTEXT: usize = 3;
/// Nejvíc řádků diffu v kartě; zbytek se jen spočítá.
pub const MAX_LINES: usize = 400;
/// Nejvíc editací, které Myers ještě hledá (pak `rough`).
pub const MAX_D: usize = 1000;
/// Delší řádek se v kartě zkrátí (znaky).
const LINE_CHARS: usize = 400;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Eq,
    Del,
    Add,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Ctx,
    Add,
    Del,
    /// vynechané nezměněné řádky
    Gap,
}

/// Řádek karty. Čísla jsou 1-based v původním (`old`) / novém (`new`) textu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Line {
    pub kind: Kind,
    pub old: Option<u32>,
    pub new: Option<u32>,
    pub text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rendered {
    pub lines: Vec<Line>,
    pub added: u32,
    pub removed: u32,
    /// kolik řádků se za stropem neukázalo
    pub cut: u32,
}

/// Operace, které z `a` udělají `b`; druhá hodnota `true` = rozdíl byl nad `max_d` a prostředek je hrubé
/// nahrazení (pořád správné, jen ne nejkratší).
pub fn ops(a: &[&str], b: &[&str], max_d: usize) -> (Vec<Op>, bool) {
    let pre = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let (ra, rb) = (&a[pre..], &b[pre..]);
    let suf = ra.iter().rev().zip(rb.iter().rev()).take_while(|(x, y)| x == y).count();
    let (ma, mb) = (&ra[..ra.len() - suf], &rb[..rb.len() - suf]);
    let mut out = vec![Op::Eq; pre];
    let rough = match myers(ma, mb, max_d) {
        Some(mid) => {
            out.extend(mid);
            false
        }
        None => {
            out.resize(out.len() + ma.len(), Op::Del);
            out.resize(out.len() + mb.len(), Op::Add);
            true
        }
    };
    out.resize(out.len() + suf, Op::Eq);
    (out, rough)
}

/// Myers „furthest reaching D-path“; `trace[d]` = V po kroku `d` pro k ∈ −d..=d.
fn myers(a: &[&str], b: &[&str], max_d: usize) -> Option<Vec<Op>> {
    let (n, m) = (a.len() as isize, b.len() as isize);
    if n == 0 {
        return Some(vec![Op::Add; m as usize]);
    }
    if m == 0 {
        return Some(vec![Op::Del; n as usize]);
    }
    let max = (n + m) as usize;
    let off = max as isize + 1;
    let mut v = vec![0isize; 2 * max + 3];
    let mut trace: Vec<Vec<isize>> = Vec::new();
    for d in 0..=(max.min(max_d) as isize) {
        let mut done = false;
        for k in (-d..=d).step_by(2) {
            let i = (k + off) as usize;
            let mut x = if k == -d || (k != d && v[i - 1] < v[i + 1]) { v[i + 1] } else { v[i - 1] + 1 };
            let mut y = x - k;
            while x < n && y < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v[i] = x;
            if x >= n && y >= m {
                done = true;
                break;
            }
        }
        trace.push(v[(off - d) as usize..=(off + d) as usize].to_vec());
        if done {
            return Some(backtrack(&trace, n, m));
        }
    }
    None
}

fn backtrack(trace: &[Vec<isize>], n: isize, m: isize) -> Vec<Op> {
    let mut out = Vec::with_capacity((n + m) as usize);
    let (mut x, mut y) = (n, m);
    for d in (1..trace.len() as isize).rev() {
        let prev = &trace[(d - 1) as usize];
        let get = |k: isize| prev[(k + d - 1) as usize];
        let k = x - y;
        let pk = if k == -d || (k != d && get(k - 1) < get(k + 1)) { k + 1 } else { k - 1 };
        let px = get(pk);
        let py = px - pk;
        while x > px && y > py {
            out.push(Op::Eq);
            x -= 1;
            y -= 1;
        }
        if x == px {
            out.push(Op::Add);
            y -= 1;
        } else {
            out.push(Op::Del);
            x -= 1;
        }
    }
    while x > 0 && y > 0 {
        out.push(Op::Eq);
        x -= 1;
        y -= 1;
    }
    out.reverse();
    out
}

pub(crate) fn clip(s: &str) -> String {
    if s.chars().count() > LINE_CHARS { s.chars().take(LINE_CHARS).collect::<String>() + "…" } else { s.to_string() }
}

/// Řádky karty z operací. `first` = číslo prvního řádku `a` i `b` (1 pro celý soubor).
pub fn render(a: &[&str], b: &[&str], ops: &[Op], first: u32, max_lines: usize) -> Rendered {
    let n = ops.len();
    // vzdálenost k nejbližší změně zleva a zprava → co je kontext
    let mut near = vec![usize::MAX; n];
    let mut last = None;
    for (p, o) in ops.iter().enumerate() {
        if *o != Op::Eq {
            last = Some(p);
        }
        if let Some(l) = last {
            near[p] = p - l;
        }
    }
    let mut next = None;
    for p in (0..n).rev() {
        if ops[p] != Op::Eq {
            next = Some(p);
        }
        if let Some(q) = next {
            near[p] = near[p].min(q - p);
        }
    }
    let mut r = Rendered::default();
    let (mut i, mut j) = (0usize, 0usize);
    let mut gap = false;
    let mut shown = 0usize;
    let mut push = |r: &mut Rendered, line: Line| {
        if shown < max_lines {
            r.lines.push(line);
        } else {
            r.cut += 1;
        }
        shown += 1;
    };
    let num = |x: usize| Some(first + x as u32);
    for (p, o) in ops.iter().enumerate() {
        match o {
            Op::Eq if near[p] > CONTEXT => {
                if !gap {
                    push(&mut r, Line { kind: Kind::Gap, old: None, new: None, text: String::new() });
                }
                gap = true;
            }
            Op::Eq => {
                gap = false;
                push(&mut r, Line { kind: Kind::Ctx, old: num(i), new: num(j), text: clip(a[i]) });
            }
            Op::Del => {
                gap = false;
                r.removed += 1;
                push(&mut r, Line { kind: Kind::Del, old: num(i), new: None, text: clip(a[i]) });
            }
            Op::Add => {
                gap = false;
                r.added += 1;
                push(&mut r, Line { kind: Kind::Add, old: None, new: num(j), text: clip(b[j]) });
            }
        }
        match o {
            Op::Eq => {
                i += 1;
                j += 1;
            }
            Op::Del => i += 1,
            Op::Add => j += 1,
        }
    }
    // bez změny není co ukázat (ani osamocená mezera)
    if r.added + r.removed == 0 {
        return Rendered::default();
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (popis, a, b, tvar karty, +, −)
    type DiffCase<'a> = (&'a str, &'a str, &'a str, Vec<&'a str>, u32, u32);

    fn lines(s: &str) -> Vec<&str> {
        s.lines().collect()
    }
    /// Zápis karty: `+x`, `-x`, ` x`, `…` — pro čitelné tabulky.
    fn shape(r: &Rendered) -> Vec<String> {
        r.lines
            .iter()
            .map(|l| match l.kind {
                Kind::Ctx => format!(" {}", l.text),
                Kind::Add => format!("+{}", l.text),
                Kind::Del => format!("-{}", l.text),
                Kind::Gap => "…".into(),
            })
            .collect()
    }
    fn apply(a: &[&str], b: &[&str], ops: &[Op]) -> Vec<String> {
        let (mut i, mut j, mut out) = (0, 0, vec![]);
        for o in ops {
            match o {
                Op::Eq => {
                    assert_eq!(a[i], b[j]);
                    out.push(a[i].to_string());
                    i += 1;
                    j += 1;
                }
                Op::Del => i += 1,
                Op::Add => {
                    out.push(b[j].to_string());
                    j += 1;
                }
            }
        }
        assert_eq!((i, j), (a.len(), b.len()));
        out
    }
    /// LCS přes DP — referenční minimum editací pro malé vstupy.
    fn min_edits(a: &[&str], b: &[&str]) -> usize {
        let mut t = vec![vec![0usize; b.len() + 1]; a.len() + 1];
        for i in (0..a.len()).rev() {
            for j in (0..b.len()).rev() {
                t[i][j] = if a[i] == b[j] { t[i + 1][j + 1] + 1 } else { t[i + 1][j].max(t[i][j + 1]) };
            }
        }
        a.len() + b.len() - 2 * t[0][0]
    }

    #[test]
    fn diff_table() {
        let cases: Vec<DiffCase> = vec![
            ("obojí prázdné", "", "", vec![], 0, 0),
            ("beze změny", "a\nb", "a\nb", vec![], 0, 0),
            ("jen přidání do prázdného", "", "x\ny", vec!["+x", "+y"], 2, 0),
            ("jen odebrání všeho", "x\ny", "", vec!["-x", "-y"], 0, 2),
            ("přidání na konec", "a\nb", "a\nb\nc", vec![" a", " b", "+c"], 1, 0),
            ("odebrání ze začátku", "a\nb\nc", "b\nc", vec!["-a", " b", " c"], 0, 1),
            (
                "změna uprostřed s kontextem 3 a mezerami",
                "1\n2\n3\n4\n5\n6\n7\n8\n9",
                "1\n2\n3\n4\nX\n6\n7\n8\n9",
                vec!["…", " 2", " 3", " 4", "-5", "+X", " 6", " 7", " 8", "…"],
                1,
                1,
            ),
            (
                "dvě vzdálené změny = dva úseky",
                "a\n1\n2\n3\n4\n5\n6\n7\n8\nb",
                "A\n1\n2\n3\n4\n5\n6\n7\n8\nB",
                vec!["-a", "+A", " 1", " 2", " 3", "…", " 6", " 7", " 8", "-b", "+B"],
                2,
                2,
            ),
        ];
        for (what, a, b, want, add, del) in cases {
            let (a, b) = (lines(a), lines(b));
            let (o, rough) = ops(&a, &b, MAX_D);
            assert!(!rough, "{what}");
            let r = render(&a, &b, &o, 1, MAX_LINES);
            assert_eq!(shape(&r), want, "{what}");
            assert_eq!((r.added, r.removed, r.cut), (add, del, 0), "{what}");
        }
    }

    #[test]
    fn line_numbers_follow_both_sides() {
        let (a, b) = (lines("a\nb\nc"), lines("a\nX\nY\nc"));
        let (o, _) = ops(&a, &b, MAX_D);
        let r = render(&a, &b, &o, 10, MAX_LINES);
        let nums: Vec<(Option<u32>, Option<u32>)> = r.lines.iter().map(|l| (l.old, l.new)).collect();
        assert_eq!(nums, [(Some(10), Some(10)), (Some(11), None), (None, Some(11)), (None, Some(12)), (Some(12), Some(13))]);
    }

    #[test]
    fn cap_counts_the_rest() {
        let b: Vec<String> = (0..1000).map(|i| format!("r{i}")).collect();
        let b: Vec<&str> = b.iter().map(String::as_str).collect();
        let (o, _) = ops(&[], &b, MAX_D);
        let r = render(&[], &b, &o, 1, MAX_LINES);
        assert_eq!((r.lines.len(), r.cut, r.added), (MAX_LINES, 600, 1000));
    }

    #[test]
    fn huge_rewrite_is_rough_but_correct() {
        let a: Vec<String> = (0..50).map(|i| format!("a{i}")).collect();
        let b: Vec<String> = (0..50).map(|i| format!("b{i}")).collect();
        let (a, b): (Vec<&str>, Vec<&str>) = (a.iter().map(String::as_str).collect(), b.iter().map(String::as_str).collect());
        let (o, rough) = ops(&a, &b, 10);
        assert!(rough);
        assert_eq!(apply(&a, &b, &o), b);
        let (o, rough) = ops(&a, &b, MAX_D);
        assert!(!rough);
        assert_eq!(o.iter().filter(|x| **x != Op::Eq).count(), 100);
    }

    #[test]
    fn long_line_is_clipped() {
        let long = "x".repeat(LINE_CHARS + 10);
        let (a, b) = (vec![], vec![long.as_str()]);
        let (o, _) = ops(&a, &b, MAX_D);
        let r = render(&a, &b, &o, 1, MAX_LINES);
        assert_eq!(r.lines[0].text.chars().count(), LINE_CHARS + 1);
    }

    /// Náhodné malé vstupy: výsledek vždy dá `b` a je nejkratší (proti LCS).
    #[test]
    fn myers_matches_lcs_on_random_inputs() {
        let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut rnd = |n: u64| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            s % n
        };
        let alpha = ["a", "b", "c", "d"];
        for _ in 0..2000 {
            let la = rnd(9) as usize;
            let lb = rnd(9) as usize;
            let a: Vec<&str> = (0..la).map(|_| alpha[rnd(4) as usize]).collect();
            let b: Vec<&str> = (0..lb).map(|_| alpha[rnd(4) as usize]).collect();
            let (o, rough) = ops(&a, &b, MAX_D);
            assert!(!rough);
            assert_eq!(apply(&a, &b, &o), b, "{a:?} → {b:?}");
            assert_eq!(o.iter().filter(|x| **x != Op::Eq).count(), min_edits(&a, &b), "{a:?} → {b:?}");
        }
    }
}
