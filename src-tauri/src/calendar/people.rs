//! Účastníci schůzky (L85): jméno / e-mail a stav odpovědi z toho, co zdroj už vrací (Graph `attendees`,
//! Google `attendees`, ICS `ATTENDEE`, Outlook `RequiredAttendees`) — bez nového oprávnění. Odmítnutí a
//! místnosti / zařízení se nekreslí; zdroj bez účastníků = prázdný seznam (UI avatary nekreslí).

use serde::{Deserialize, Serialize};

/// Nejvíc uložených účastníků na událost (zbytek jen v `Crowd.more`).
pub const MAX_PEOPLE: usize = 8;
/// Nejvíc účastníků, které zdroj před oříznutím zpracujeme (strop práce i paměti u obřích pozvánek).
pub const MAX_RAW: usize = 60;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Resp {
    /// zatím bez odpovědi / zdroj odpověď nezná
    #[default]
    Pending,
    Accepted,
    Tentative,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Person {
    /// zobrazované jméno; prázdné = zdroj ho nedal (UI použije e-mail)
    #[serde(default)]
    pub name: String,
    /// malým písmem; prázdný = zdroj ho nedal (Outlook COM)
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub resp: Resp,
}

/// Účastníci události: prvních `MAX_PEOPLE` a počet dalších.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Crowd {
    #[serde(default)]
    pub list: Vec<Person>,
    #[serde(default)]
    pub more: u16,
}

impl Crowd {
    pub fn is_empty(&self) -> bool {
        self.list.is_empty() && self.more == 0
    }

    /// Z přijatých osob: prázdné vyhodí, duplicity (stejný e-mail, jinak stejné jméno bez ohledu na velikost)
    /// sloučí, ořízne na `MAX_PEOPLE`.
    pub fn of(people: impl IntoIterator<Item = Person>) -> Crowd {
        let mut list: Vec<Person> = Vec::new();
        let mut seen: Vec<String> = Vec::new();
        for p in people.into_iter().take(MAX_RAW) {
            let name = super::clip(&p.name, 60);
            let email = super::clip(&p.email, 80).to_lowercase();
            let key = if email.is_empty() { name.to_lowercase() } else { email.clone() };
            if key.is_empty() || seen.contains(&key) {
                continue;
            }
            seen.push(key);
            list.push(Person { name, email, resp: p.resp });
        }
        let more = list.len().saturating_sub(MAX_PEOPLE).min(u16::MAX as usize) as u16;
        list.truncate(MAX_PEOPLE);
        Crowd { list, more }
    }
}

/// Stav odpovědi ze zdroje (Graph `accepted` / `tentativelyAccepted` / `declined` / `none` / `notResponded` / `organizer`,
/// Google `needsAction` / `tentative`, ICS `PARTSTAT`). `None` = odmítl → nezobrazovat.
pub fn resp(s: &str) -> Option<Resp> {
    match s.trim().to_ascii_lowercase().as_str() {
        "declined" => None,
        "accepted" | "organizer" => Some(Resp::Accepted),
        "tentative" | "tentativelyaccepted" => Some(Resp::Tentative),
        _ => Some(Resp::Pending),
    }
}

/// Osoba ze zdroje: `None` = odmítl.
pub fn person(name: &str, email: &str, status: &str) -> Option<Person> {
    resp(status).map(|resp| Person { name: name.trim().to_string(), email: email.trim().to_string(), resp })
}

/// Seznam jmen z řetězce Outlooku (`RequiredAttendees`: „Jan Novák; Eva Svobodová“) — jen jména, stav neznáme.
pub fn from_names(s: &str) -> Vec<Person> {
    s.split(';').map(str::trim).filter(|n| !n.is_empty()).map(|n| Person { name: n.to_string(), ..Person::default() }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(name: &str, email: &str) -> Person {
        Person { name: name.into(), email: email.into(), ..Person::default() }
    }

    #[test]
    fn resp_table() {
        for (s, want) in [
            ("declined", None),
            ("Declined", None),
            ("accepted", Some(Resp::Accepted)),
            ("organizer", Some(Resp::Accepted)),
            ("tentativelyAccepted", Some(Resp::Tentative)),
            ("tentative", Some(Resp::Tentative)),
            ("needsAction", Some(Resp::Pending)),
            ("none", Some(Resp::Pending)),
            ("", Some(Resp::Pending)),
        ] {
            assert_eq!(resp(s), want, "{s}");
        }
    }

    #[test]
    fn crowd_dedup_cap_and_empty() {
        // duplicita podle e-mailu (velikost písmen), bez e-mailu podle jména, prázdné pryč
        let c = Crowd::of([p("Tomáš", "Tomas@Firma.cz"), p("Tomáš Hora", "tomas@firma.cz"), p("Eva", ""), p("EVA", ""), p("", ""), p("", "x@y.cz")]);
        let got: Vec<(&str, &str)> = c.list.iter().map(|x| (x.name.as_str(), x.email.as_str())).collect();
        assert_eq!(got, [("Tomáš", "tomas@firma.cz"), ("Eva", ""), ("", "x@y.cz")]);
        assert_eq!(c.more, 0);
        // strop: 8 + zbytek
        let c = Crowd::of((0..11).map(|i| p(&format!("Os{i}"), "")));
        assert_eq!((c.list.len(), c.more), (MAX_PEOPLE, 3));
        assert!(Crowd::of([]).is_empty());
    }

    #[test]
    fn outlook_names() {
        let v = from_names("Jan Novák; Eva Svobodová;; ");
        assert_eq!(v.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), ["Jan Novák", "Eva Svobodová"]);
        assert!(from_names("").is_empty());
    }
}
