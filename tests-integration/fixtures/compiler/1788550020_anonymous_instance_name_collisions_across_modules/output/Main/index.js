import * as Library from "../Library/index.js";
export function pick(dictionary) {
  return dictionary.pick;
}
const pickDictPick = /* @__PURE__ */ Library.pick(Library.pick5);
export const pick1 = { pick: ($proxy) => 10 | 0 };
export const result = {
  third: /* @__PURE__ */ pickDictPick("Proxy"),
  first: /* @__PURE__ */ Library.pick(Library.pick2)("Proxy"),
  second: /* @__PURE__ */ Library.pick(Library.pick3)("Proxy"),
  named: /* @__PURE__ */ Library.pick(Library.pick4)("Proxy"),
  repeated: /* @__PURE__ */ pickDictPick("Proxy"),
  local: /* @__PURE__ */ pick(pick1)("Proxy")
};
