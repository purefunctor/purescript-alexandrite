import * as $foreign from "./foreign.js";
export function show(dictionary) {
  return dictionary.show;
}
export const consume = $foreign["consume"];
export const evidenceDoesNotEscape = (() => {
  const $field = (eqADict) => /* @__PURE__ */ show(eqADict);
  let $result;
  throw new Error("Generated code reached a source error");
  return {
    scoped: $field,
    leaked: /* @__PURE__ */ show($result)
  };
})();
export const typeDoesNotEscape = consume({ value: (value) => value });
