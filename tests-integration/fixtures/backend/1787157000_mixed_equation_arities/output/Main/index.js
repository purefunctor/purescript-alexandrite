export function choose($int) {
  return $int$1 => {
    if ($int === (0 | 0)) {
      function choose$closure(value) {
        return value;
      }
      return choose$closure($int$1);
    } else {
      return $int;
    }
  };
}
