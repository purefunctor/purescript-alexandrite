export function eqArray(eqADict) {
  function eqArray$closure($array) {
    return $array$1 => {
      return true;
    };
  }
  return { eq: eqArray$closure };
}

export const eqInt = (() => {
  function eqInt$initialize$closure($int) {
    return $int$1 => {
      return true;
    };
  }
  return { eq: eqInt$initialize$closure };
})();

export const eqBoolean = (() => {
  function eqBoolean$initialize$closure($boolean) {
    return $boolean$1 => {
      return true;
    };
  }
  return { eq: eqBoolean$initialize$closure };
})();

export const orderedInt = (() => {
  function orderedInt$initialize$closure(unit) {
    return eqInt;
  }
  function orderedInt$initialize$closure$1($int) {
    return $int$1 => {
      return true;
    };
  }
  return { Eq0: orderedInt$initialize$closure, lessThanOrEqual: orderedInt$initialize$closure$1 };
})();
