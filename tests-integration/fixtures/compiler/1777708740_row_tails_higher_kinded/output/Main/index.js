import * as $foreign from "./foreign.js";
export const unify = $foreign["unify"];
export const withF = $foreign["withF"];
export const withG = $foreign["withG"];
export const test = unify(withF)(withG);
