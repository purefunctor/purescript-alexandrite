export function headOr($a) {
  return ($array) => {
    if (Array.isArray($array) && $array.length === 2) {
      const fallback = $a;
      const first = $array[0];
      return first;
    }
    const fallback$1 = $a;
    return fallback$1;
  };
}
