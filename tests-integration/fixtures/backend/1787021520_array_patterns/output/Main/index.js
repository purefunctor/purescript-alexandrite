export function describe($array) {
  if (Array.isArray($array) && $array.length === 0) {
    return 0 | 0;
  } else {
    if (Array.isArray($array) && $array.length === 1) {
      return $array[0];
    } else {
      if (Array.isArray($array) && $array.length === 2) {
        const first = $array[0];
        const second = $array[1];
        return first;
      } else {
        return 3 | 0;
      }
    }
  }
}
