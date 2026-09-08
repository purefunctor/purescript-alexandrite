import * as $foreign from "./foreign.js";
export function example(dictionary) {
  return dictionary.example;
}
export function implementation(failTextDict) {
  return (requiredIntDict) => {
    return (failTextDict$1) => {
      return boolean;
    };
  };
}
export function exampleInt(failTextDict) {
  return (requiredIntDict) => {
    return (failTextDict$1) => {
      return { example: /* @__PURE__ */ implementation(failTextDict)(requiredIntDict)(failTextDict$1) };
    };
  };
}
export const boolean = $foreign["boolean"];
