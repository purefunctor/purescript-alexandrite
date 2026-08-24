import * as $foreign from "./foreign.js";

export const intSubtract = $foreign["intSubtract"];
export const intNegate = $foreign["intNegate"];

export const ringInt = { sub: intSubtract, negate: intNegate };
