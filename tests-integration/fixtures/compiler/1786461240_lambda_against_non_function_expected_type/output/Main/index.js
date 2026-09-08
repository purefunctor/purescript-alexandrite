import * as $foreign from "./foreign.js";
export const Unit = "Unit";
export const consume = $foreign["consume"];
export const test = consume((value) => 0 | 0);
