export function identity(a) {
  return a;
}
function $const(a) {
  return ($b) => {
    return a;
  };
}
export const testIdentity = identity;
export const testConst = $const;
export { $const as "const" };
