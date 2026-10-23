// Angličtina pro útratu Claude (odhad v cenách API; `src/costs/*`, sekce v kartě Claude; klíče `sessions.costs.*`).
import { registerDict } from "../i18n";

registerDict("en", {
  "sessions.costs.usd": "${v}",
  "sessions.costs.other": "other",
  "sessions.costs.noDir": "no folder",
  "sessions.costs.unpricedTip": "{n} tokens without a price in the price list",
  "sessions.costs.today": "Today",
  "sessions.costs.week": "7 days",
  "sessions.costs.window": "{n} days",
  "sessions.costs.title": "Value at API prices",
  "sessions.costs.tip": "An estimate from the API price list — you don't pay it on a subscription.",
  "sessions.costs.todayShort": "today {v}",
  "sessions.costs.tagTip": "A Pro/Max subscription isn't billed per token. API price list checked {date}: {src}",
  "sessions.costs.tag": "estimate at API prices, not a bill",
  "sessions.costs.computingPct": "counting… {n} %",
  "sessions.costs.computing": "counting…",
  "sessions.costs.tokens": "{n} tokens",
  "sessions.costs.split": "input {i} · output {o} · cache write {w} · cache read {r}",
  "sessions.costs.unpriced": "{n} tokens without a price (model missing from the price list)",
  "sessions.costs.barsLabel": "Spend per day",
  "sessions.costs.empty": "No Claude replies in this period.",
  "sessions.costs.projects": "Projects",
  "sessions.costs.models": "Models",
  "sessions.costs.computed": "counted {when}",
});
