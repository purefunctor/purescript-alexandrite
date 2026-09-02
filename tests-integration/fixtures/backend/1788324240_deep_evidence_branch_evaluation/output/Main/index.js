import * as $foreign from "./foreign.js";
export const Proxy = "Proxy";
export function built(dictionary) {
  return dictionary.built;
}
export function buildNext(addPreviousCurrentDict) {
  return (buildPreviousDict) => {
    return { built: crash(0 | 0) };
  };
}
export function buildValue(buildNumberDict) {
  return ($proxy) => /* @__PURE__ */ built(buildNumberDict);
}
export function evaluateIf(condition) {
  if (condition) {
    const buildNextDict = /* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(buildZero)))))))))))))))))))))))))))))));
    const buildNextDict$1 = /* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(buildNextDict))))))))))))))))))))))))))))))));
    const $result = /* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(buildNextDict$1)))))));
    return /* @__PURE__ */ buildValue($result)("Proxy");
  } else {
    return 0 | 0;
  }
}
export function evaluateCase(condition) {
  if (condition === true) {
    const buildNextDict = /* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(buildZero)))))))))))))))))))))))))))))));
    const buildNextDict$1 = /* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(buildNextDict))))))))))))))))))))))))))))))));
    const $result = /* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(buildNextDict$1)))))));
    return /* @__PURE__ */ buildValue($result)("Proxy");
  }
  if (condition === false) {
    return 0 | 0;
  }
  throw new Error("Pattern match failure");
}
export function evaluateGuard(condition) {
  if (condition) {
    const buildNextDict = /* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(buildZero)))))))))))))))))))))))))))))));
    const buildNextDict$1 = /* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(buildNextDict))))))))))))))))))))))))))))))));
    const $result = /* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(/* @__PURE__ */ buildNext({})(buildNextDict$1)))))));
    return /* @__PURE__ */ buildValue($result)("Proxy");
  }
  if (true) {
    return 0 | 0;
  }
  throw new Error("Pattern match failure");
}
export const crash = $foreign["crash"];
export const buildZero = { built: 0 | 0 };
