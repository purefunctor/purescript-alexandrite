export function identity(value) {
  return value;
}
function $const(value) {
  return ($b) => {
    return value;
  };
}
export const testIdentity = identity;
export const testConst = $const;
export { $const as "const" };
