import * as $foreign from "./foreign.js";
export const Unit = "Unit";
export const pure = $foreign["pure"];
export const discard = $foreign["discard"];
export const use1 = $foreign["use1"];
export const use2 = $foreign["use2"];
export const test = discard(use1)(($unit) => discard(use2)(($unit$1) => pure("Unit")));
