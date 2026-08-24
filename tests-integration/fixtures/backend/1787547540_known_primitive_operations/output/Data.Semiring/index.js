import * as $foreign from "./foreign.js";

export const intAdd = $foreign["intAdd"];
export const intMultiply = $foreign["intMultiply"];

export const semiringInt = { add: intAdd, mul: intMultiply };
