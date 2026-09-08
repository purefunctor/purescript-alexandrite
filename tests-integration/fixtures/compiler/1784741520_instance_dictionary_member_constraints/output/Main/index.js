import * as $foreign from "./foreign.js";
export function apply(dictionary) {
  return dictionary.apply;
}
export function implementation(requiredBDict) {
  return ($a) => ($b) => boolean;
}
export const boolean = $foreign["boolean"];
export const exampleInt = { apply: (requiredBDict) => /* @__PURE__ */ implementation(requiredBDict) };
