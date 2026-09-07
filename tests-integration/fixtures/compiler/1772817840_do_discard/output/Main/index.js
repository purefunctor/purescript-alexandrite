import * as $foreign from "./foreign.js";
export const Unit = "Unit";
export const pure = $foreign["pure"];
export const bind = $foreign["bind"];
export const discard = $foreign["discard"];
export const unit = "Unit";
export const test = discard(pure(1 | 0))(($int) => pure(unit));
const test_ = discard(pure(1 | 0))(($int) => pure(unit));
export { test_ as "test'" };
