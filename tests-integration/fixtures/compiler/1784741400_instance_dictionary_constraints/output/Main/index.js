import * as $foreign from "./foreign.js";
export function example(dictionary) {
  return dictionary.example;
}
export function implementation(secondBDict) {
  return ($a) => ($b) => boolean;
}
export function exampleAB(firstADict) {
  return (secondBDict) => {
    return { example: /* @__PURE__ */ implementation(secondBDict) };
  };
}
export const boolean = $foreign["boolean"];
