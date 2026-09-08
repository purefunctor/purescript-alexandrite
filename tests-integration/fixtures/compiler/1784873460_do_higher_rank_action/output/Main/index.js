import * as $foreign from "./foreign.js";
export const polymorphic = $foreign["polymorphic"];
export const bind = $foreign["bind"];
export const higherRank = bind(polymorphic)((value) => 42 | 0);
