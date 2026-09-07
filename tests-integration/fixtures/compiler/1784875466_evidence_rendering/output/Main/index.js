import * as $foreign from "./foreign.js";
export function childInt(parentIntDict) {
  return { Parent0: () => parentIntDict };
}
export function superclassEvidence(childValueDict) {
  return /* @__PURE__ */ useParent(/* @__PURE__ */ childValueDict.Parent0());
}
export const useParent = $foreign["useParent"];
export const useChild = $foreign["useChild"];
export const parentInt = {};
export const instanceEvidence = /* @__PURE__ */ useChild(/* @__PURE__ */ childInt(parentInt))(1 | 0);
