export function useLacksEmpty(lacksNameRowDict) {
  return ($proxy) => 0 | 0;
}
export function forwardLacksEmpty(p) {
  return /* @__PURE__ */ useLacksEmpty({})(p);
}
