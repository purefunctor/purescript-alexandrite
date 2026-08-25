export function describe($array) {
  if (Array.isArray($array) && $array.length === 0) {
    return 0 | 0;
  }
  if (Array.isArray($array) && $array.length === 1) {
    const value = $array[0];
    return value;
  }
  if (Array.isArray($array) && $array.length === 2) {
    const first = $array[0];
    const second = $array[1];
    return first;
  }
  return 3 | 0;
}
