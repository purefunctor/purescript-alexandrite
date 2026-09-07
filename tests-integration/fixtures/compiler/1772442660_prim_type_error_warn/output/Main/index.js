export const Proxy = "Proxy";
export function warnBasic(warnTextDict) {
  return (x) => x;
}
export function warnBeside(warnBesideTextTextDict) {
  return (x) => x;
}
export function warnAbove(warnAboveTextTextDict) {
  return (x) => x;
}
export function warnQuote(warnBesideTextQuoteADict) {
  return (p) => p;
}
export function warnQuoteLabel(warnBesideTextQuoteLabelDict) {
  return (x) => x;
}
export function warnQuoteLabelSpaces(warnBesideTextQuoteLabelDict) {
  return (x) => x;
}
export function warnQuoteLabelQuote(warnBesideTextQuoteLabelDict) {
  return (x) => x;
}
export function warnQuoteLabelRaw(warnBesideTextQuoteLabelDict) {
  return (x) => x;
}
export const useWarnBasic = /* @__PURE__ */ warnBasic({})(42 | 0);
export const useWarnBeside = /* @__PURE__ */ warnBeside({})(42 | 0);
export const useWarnAbove = /* @__PURE__ */ warnAbove({})(42 | 0);
export const useWarnQuote = /* @__PURE__ */ warnQuote({})("Proxy");
export const useWarnQuoteLabel = /* @__PURE__ */ warnQuoteLabel({})(42 | 0);
export const useWarnQuoteLabelSpaces = /* @__PURE__ */ warnQuoteLabelSpaces({})(42 | 0);
export const useWarnQuoteLabelQuote = /* @__PURE__ */ warnQuoteLabelQuote({})(42 | 0);
export const useWarnQuoteLabelRaw = /* @__PURE__ */ warnQuoteLabelRaw({})(42 | 0);
