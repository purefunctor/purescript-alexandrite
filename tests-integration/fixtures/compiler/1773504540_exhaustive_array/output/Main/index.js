export function testNonExhaustive($array) {
  if (Array.isArray($array) && $array.length === 0) {
    return 0 | 0;
  }
  if (Array.isArray($array) && $array.length === 1) {
    const x = $array[0];
    return x;
  }
  throw new Error("Pattern match failure");
}
export function testRedundant($array) {
  if (Array.isArray($array) && $array.length === 1) {
    const x = $array[0];
    return x;
  }
  if (Array.isArray($array) && $array.length === 1) {
    const y = $array[0];
    return y;
  }
  return 0 | 0;
}
export function testExhaustive($array) {
  if (Array.isArray($array) && $array.length === 0) {
    return 0 | 0;
  }
  if (Array.isArray($array) && $array.length === 1) {
    const x = $array[0];
    return x;
  }
  return 0 | 0;
}
