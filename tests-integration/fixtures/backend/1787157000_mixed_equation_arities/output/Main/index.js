export function choose($int) {
  return $int$1 => {
    if ($int === (0 | 0)) {
      return (value => value)($int$1);
    } else {
      return $int;
    }
  };
}
