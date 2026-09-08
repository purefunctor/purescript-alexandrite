import * as $foreign from "./foreign.js";
export const Unit = "Unit";
export const consume = $foreign["consume"];
export const use = $foreign["use"];
export const sectioned = consume((section36) => (partialDict) => (section38) => use(section36)(section38));
export const lowered = consume((unit) => (partialDict) => (value) => use(unit)(value));
