export function identity(value) {
  return value;
}
export function consume(value) {
  return value;
}
export const empty = [];
export const values = [1 | 0, 2 | 0];
export const nested = [[1 | 0], [2 | 0, 3 | 0]];
export const inferredFunctions = [identity];
export const checkedFunctions = [identity];
export const checkedIdentity = identity;
export const application = consume([1 | 0, 2 | 0]);
