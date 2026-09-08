import * as $foreign from "./foreign.js";
export function left(dictionary) {
  return dictionary.left;
}
export function leftZero(seedDict) {
  return { left: observe("left") };
}
export function leftNext(addPreviousCurrentDict) {
  return (leftPreviousDict) => {
    return { left: 0 | 0 };
  };
}
export function right(dictionary) {
  return dictionary.right;
}
export function rightZero(seedDict) {
  return { right: observe("right") };
}
export function rightNext(addPreviousCurrentDict) {
  return (rightPreviousDict) => {
    return { right: 0 | 0 };
  };
}
export function combined(dictionary) {
  return dictionary.combined;
}
export function combinedInstance(leftDict) {
  return (rightDict) => {
    return { combined: 0 | 0 };
  };
}
export const observe = $foreign["observe"];
export const seed = {};
export const result = /* @__PURE__ */ (() => {
  const leftNextDict = /* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftZero(seed)))))))))))))))))))))))))))))));
  const leftNextDict$1 = /* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(leftNextDict))))))))))))))))))))))))))))))));
  const rightNextDict = /* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightZero(seed)))))))))))))))))))))))))))))));
  const $result = /* @__PURE__ */ combinedInstance(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(/* @__PURE__ */ leftNext({})(leftNextDict$1)))))))))(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(/* @__PURE__ */ rightNext({})(rightNextDict)))))))))));
  return /* @__PURE__ */ combined($result);
})();
