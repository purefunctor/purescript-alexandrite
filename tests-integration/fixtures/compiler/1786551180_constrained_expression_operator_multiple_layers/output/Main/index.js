import * as $foreign from "./foreign.js";
export const apply = $foreign["apply"];
export const multiplyConstrained = $foreign["multiplyConstrained"];
export const consumeMultiplyConstrained = $foreign["consumeMultiplyConstrained"];
export const multipleConstraintLayers = apply(consumeMultiplyConstrained)((firstIntDict) => (secondIntDict) => /* @__PURE__ */ multiplyConstrained(firstIntDict)(secondIntDict));
