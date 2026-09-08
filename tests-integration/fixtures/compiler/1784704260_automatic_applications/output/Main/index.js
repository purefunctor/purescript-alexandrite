import * as $foreign from "./foreign.js";
function $default(dictionary) {
  return dictionary.default;
}
export function useDefault(defaultADict) {
  return /* @__PURE__ */ $default(defaultADict);
}
export function useInterleaved(firstADict) {
  return (secondBDict) => {
    return /* @__PURE__ */ interleaved(firstADict)(secondBDict);
  };
}
export const interleaved = $foreign["interleaved"];
export { $default as "default" };
