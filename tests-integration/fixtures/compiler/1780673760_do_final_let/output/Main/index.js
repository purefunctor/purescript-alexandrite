import * as $foreign from "./foreign.js";
export const bind = $foreign["bind"];
export const before = $foreign["before"];
export const expectString = $foreign["expectString"];
export const test = /* @__PURE__ */ (() => {
  const $closure = (record) => {
    const $scrutinee = expectString(record.field);
    throw new Error("Generated code reached a source error");
  };
  return bind(before)($closure);
})();
