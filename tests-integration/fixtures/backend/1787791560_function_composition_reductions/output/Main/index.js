import * as Control_Semigroupoid from "../Control.Semigroupoid/index.js";
import * as Lookalike from "../Lookalike/index.js";
import * as $foreign from "./foreign.js";
export function composeOrder($boolean) {
  return /* @__PURE__ */ semigroupoidFunctionDictCompose(observe("outer")((value) => value))(observe("inner")((value$1) => value$1))(observe("argument")(42 | 0));
}
export function partiallyComposedOrder($boolean) {
  const composedFunction = /* @__PURE__ */ semigroupoidFunctionDictCompose(observe("outer")((value) => value))(observe("inner")((value$1) => value$1));
  return composedFunction(observe("argument")(42 | 0));
}
export function flippedComposeOrder($boolean) {
  return /* @__PURE__ */ Control_Semigroupoid.composeFlipped(Control_Semigroupoid.semigroupoidFn)(observe("inner")((value) => value))(observe("outer")((value$1) => value$1))(observe("argument")(42 | 0));
}
export function partiallyFlippedComposedOrder($boolean) {
  const composedFunction = /* @__PURE__ */ Control_Semigroupoid.composeFlipped(Control_Semigroupoid.semigroupoidFn)(observe("inner")((value) => value))(observe("outer")((value$1) => value$1));
  return composedFunction(observe("argument")(42 | 0));
}
export const observe = $foreign["observe"];
export const readTrace = $foreign["readTrace"];
const semigroupoidFunctionDictCompose = /* @__PURE__ */ Control_Semigroupoid.compose(Control_Semigroupoid.semigroupoidFn);
export const composed = /* @__PURE__ */ semigroupoidFunctionDictCompose((value) => value)((value$1) => value$1)(42 | 0);
export const flippedComposed = /* @__PURE__ */ Control_Semigroupoid.composeFlipped(Control_Semigroupoid.semigroupoidFn)((value) => value)((value$1) => value$1)(42 | 0);
export const lookalikeCompose = Lookalike.compose((value) => value)((value$1) => value$1)(42 | 0);
export const lookalikeComposeFlipped = Lookalike.composeFlipped((value) => value)((value$1) => value$1)(42 | 0);
