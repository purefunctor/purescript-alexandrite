import * as Data_Reflectable from "../Data.Reflectable/index.js";
export const symbol = /* @__PURE__ */ Data_Reflectable.reflectType({ reflectType: ($proxy) => "symbol" })("Proxy");
export const integer = /* @__PURE__ */ Data_Reflectable.reflectType({ reflectType: ($proxy) => 42 | 0 })("Proxy");
export const booleanTrue = /* @__PURE__ */ Data_Reflectable.reflectType({ reflectType: ($proxy) => true })("Proxy");
export const booleanFalse = /* @__PURE__ */ Data_Reflectable.reflectType({ reflectType: ($proxy) => false })("Proxy");
export const orderingLess = /* @__PURE__ */ Data_Reflectable.reflectType({ reflectType: ($proxy) => "LT" })("Proxy");
export const orderingEqual = /* @__PURE__ */ Data_Reflectable.reflectType({ reflectType: ($proxy) => "EQ" })("Proxy");
export const orderingGreater = /* @__PURE__ */ Data_Reflectable.reflectType({ reflectType: ($proxy) => "GT" })("Proxy");
