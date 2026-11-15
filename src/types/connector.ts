// Zrcadlo `connector::ConnectorConfig` (Rust, config `connector`) — propojená aplikace přes její MCP server.
// Klíč tu není (Správce pověření Windows, `Ledge/connector`).
export type ConnectorConfig = {
  /** adresa MCP serveru propojené aplikace (https; prázdná = funkce nenastavená) */
  url: string;
  /** adresa webu propojené aplikace pro odkaz na úkol v poznámce (prázdná = řádek bez odkazu) */
  appUrl: string;
  /** zobrazovaný název v menu a dialozích (prázdný = „propojená aplikace“) */
  name: string;
};

/** Výchozí stav (= `ConnectorConfig::default` v Rustu). */
export const CONNECTOR_DEFAULT: ConnectorConfig = { url: "", appUrl: "", name: "" };
