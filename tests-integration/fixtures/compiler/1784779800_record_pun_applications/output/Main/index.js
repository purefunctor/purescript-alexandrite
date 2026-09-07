import * as $foreign from "./foreign.js";
export function inferredPun(firstDict) {
  return (secondDict) => {
    return { interleaved: /* @__PURE__ */ interleaved(firstDict)(secondDict) };
  };
}
export function inferredExplicit(firstDict) {
  return (secondDict) => {
    return { interleaved: /* @__PURE__ */ interleaved(firstDict)(secondDict) };
  };
}
export const interleaved = $foreign["interleaved"];
export const fetch = $foreign["fetch"];
export const aliased = 1 | 0;
export const inferredAliasPun = { aliased };
export const inferredAliasExplicit = { aliased };
export const expectedPun = { fetch: (capabilityMDict) => /* @__PURE__ */ fetch(capabilityMDict) };
export const expectedExplicit = { fetch: (capabilityMDict) => /* @__PURE__ */ fetch(capabilityMDict) };
