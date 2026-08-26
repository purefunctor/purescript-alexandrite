import * as $foreign from "./foreign.js";
export const foreignValue = $foreign["foreignValue"];
export const foreignFunction = $foreign["foreignFunction"];
export const value = foreignFunction(foreignValue);
