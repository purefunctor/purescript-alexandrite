function $tail_f_g($state, $argument0) {
  while (true) {
    switch ($state) {
      // f
      case 0: {
        const $currentArgument0 = $argument0;
        $argument0 = $currentArgument0;
        $state = 1;
        continue;
      }
      // g
      case 1: {
        const $currentArgument0$1 = $argument0;
        $argument0 = $currentArgument0$1;
        $state = 0;
        continue;
      }
    }
  }
}
export function f(a) {
  return $tail_f_g(0, a);
}
export function g(a$1) {
  return $tail_f_g(1, a$1);
}
