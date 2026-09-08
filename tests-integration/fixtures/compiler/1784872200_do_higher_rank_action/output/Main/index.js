import * as $foreign from "./foreign.js";
export const polymorphic = $foreign["polymorphic"];
export const bind = $foreign["bind"];
export const test = bind(polymorphic)((value) => 42 | 0);
const test_ = bind(polymorphic)((value) => 42 | 0);
export { test_ as "test'" };
