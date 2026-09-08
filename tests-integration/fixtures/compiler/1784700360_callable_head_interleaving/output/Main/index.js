import * as $foreign from "./foreign.js";
export const interleaved = $foreign["interleaved"];
export const firstTypeInt = {};
export const secondTypeBoolean = {};
export const test = /* @__PURE__ */ interleaved(firstTypeInt)(secondTypeBoolean)(1 | 0)(true);
