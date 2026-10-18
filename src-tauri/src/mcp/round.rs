//! Úklid čísel ve výstupu nástrojů: `f32` z configu (`opacity`, `speed`, `fontScale`) se do JSON dostane
//! jako `f64` s šumem (`1.0499999523162842`). Model by ho opisoval a zbytečně platil tokeny, proto se
//! neceločíselná čísla zaokrouhlí na 2 desetinná místa. Config na disku se nemění — jen výstup MCP.

use serde_json::{Number, Value};

/// Zaokrouhlí na 2 desetinná místa; celá čísla a hodnoty mimo rozsah zůstanou.
pub fn two(x: f64) -> f64 {
    let r = (x * 100.0).round() / 100.0;
    if r.is_finite() { r } else { x }
}

/// Rekurzivně zaokrouhlí všechna neceločíselná čísla.
pub fn clean(v: Value) -> Value {
    match v {
        Value::Number(n) if n.is_f64() => n.as_f64().and_then(|x| Number::from_f64(two(x))).map(Value::Number).unwrap_or(Value::Number(n)),
        Value::Array(a) => Value::Array(a.into_iter().map(clean).collect()),
        Value::Object(o) => Value::Object(o.into_iter().map(|(k, x)| (k, clean(x))).collect()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rounds_float_noise_only() {
        let noisy = serde_json::to_value(1.05_f32).unwrap();
        assert_ne!(noisy.to_string(), "1.05", "f32 šum je skutečný: {noisy}");
        #[rustfmt::skip]
        let cases = [
            (json!(noisy),                       "1.05"),
            (json!(0.9_f32),                     "0.9"),
            (json!(0.333333),                    "0.33"),
            (json!(1.005),                       "1.0"),
            (json!(7),                           "7"),
            (json!(1_700_000_000_123_u64),       "1700000000123"),
            (json!(-0.126),                      "-0.13"),
            (json!("0.123456"),                  "\"0.123456\""),
            (json!(null),                        "null"),
        ];
        for (v, want) in cases {
            assert_eq!(clean(v.clone()).to_string(), want, "{v}");
        }
        let nested = clean(json!({ "a": [1.2345, { "b": serde_json::to_value(1.05_f32).unwrap() }], "n": 3 }));
        assert_eq!(nested.to_string(), r#"{"a":[1.23,{"b":1.05}],"n":3}"#);
    }
}
