import * as $foreign from "./foreign.js";
export const Unit = "Unit";
export const Token = "Token";
export const Effect = ($value0) => ({
  tag: "Effect",
  _1: $value0
});
export const Proxy = "Proxy";
export const Map = "Map";
export function identity(value) {
  return value;
}
export function modify(token) {
  return token;
}
export function validRankTwoCallback(callback) {
  return consumeIdentity(callback);
}
export function validConstrainedTypeErasingCallback(witnessKeyDict) {
  return (key) => (value) => /* @__PURE__ */ mutateMap(witnessKeyDict)(/* @__PURE__ */ pokeMap(witnessKeyDict)(key)(value));
}
export function escapedBinder(callback) {
  return consumeIdentity(callback);
}
export function escapedDiscardedOuterArgument($argument0) {
  return leak(modify);
}
export function escapedEvidenceOnly(escapingEvidenceScopeDict) {
  return leak(/* @__PURE__ */ pokeWithoutWitness(escapingEvidenceScopeDict));
}
export const consumeIdentity = $foreign["consumeIdentity"];
export const discardValue = $foreign["discardValue"];
export const leak = $foreign["leak"];
export const mutate = $foreign["mutate"];
export const mutateConstrained = $foreign["mutateConstrained"];
export const poke = $foreign["poke"];
export const pokeReturningToken = $foreign["pokeReturningToken"];
export const pokeConstrained = $foreign["pokeConstrained"];
export const pokeWithoutWitness = $foreign["pokeWithoutWitness"];
export const mutateMap = $foreign["mutateMap"];
export const pokeMap = $foreign["pokeMap"];
export const validDirectPolymorphicExpression = consumeIdentity(identity);
export const validDiscardedHigherRankResult = mutate(poke);
export const validDiscardedScopedHigherRankResult = mutate(pokeReturningToken);
export const validDiscardedConstrainedHigherRankResult = mutateConstrained((witnessScopeDict) => /* @__PURE__ */ pokeConstrained(witnessScopeDict));
export const escapedResult = leak(modify);
export const escapedNestedUse = identity(leak(modify));
export const escapedDiscardedArgument = discardValue(leak(modify));
