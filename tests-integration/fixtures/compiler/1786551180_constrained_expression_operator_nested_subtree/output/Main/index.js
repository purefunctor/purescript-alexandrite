import * as $foreign from "./foreign.js";
export const apply = $foreign["apply"];
export const identity = $foreign["identity"];
export const constrainedValue = $foreign["constrainedValue"];
export const consumeConstrained = $foreign["consumeConstrained"];
export const constrainedSubtree = apply(consumeConstrained)((firstIntDict) => apply(identity)(/* @__PURE__ */ constrainedValue(firstIntDict)));
