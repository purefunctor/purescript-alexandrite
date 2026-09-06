import * as Library from "../Library/index.js";
import * as $foreign from "./foreign.js";
export function forwarded(partialDict) {
  return (first) => (second) => ({
    first: ((wrapped) => /* @__PURE__ */ Library.unwrap(partialDict)(wrapped))(first),
    second: /* @__PURE__ */ Library.unwrap(partialDict)(second)
  });
}
export function signed(partialDict) {
  return /* @__PURE__ */ forwarded(partialDict);
}
export const unsafePartial = $foreign["unsafePartial"];
export const discharged = unsafePartial((partialDict) => /* @__PURE__ */ forwarded(partialDict));
