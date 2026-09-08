import * as $foreign from "./foreign.js";
export const apply = $foreign["apply"];
export const constrainedValue = $foreign["constrainedValue"];
export const consumeConstrained = $foreign["consumeConstrained"];
export const constrainedLeaf = apply(consumeConstrained)((firstIntDict) => /* @__PURE__ */ constrainedValue(firstIntDict));
