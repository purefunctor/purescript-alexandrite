import * as $foreign from "./foreign.js";
export function applyInterleaved(firstADict) {
  return (secondBDict) => {
    return (first) => (second) => /* @__PURE__ */ interleaved(firstADict)(first)(secondBDict)(second);
  };
}
export function layered(firstADict) {
  return (secondBDict) => {
    return (first) => ($b) => first;
  };
}
export const interleaved = $foreign["interleaved"];
