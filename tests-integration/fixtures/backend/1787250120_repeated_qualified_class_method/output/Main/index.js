import * as Data_Eq from "../Data.Eq/index.js";

export function compareTwice(eqADict) {
  function compareTwice$closure(eqADict) {
    return left => {
      return right => {
        if (eqADict.eq(left)(right)) {
          return eqADict.eq(right)(left);
        } else {
          return false;
        }
      };
    };
  }
  return compareTwice$closure(eqADict);
}

export function compareIntsTwice(left) {
  return right => {
    if (Data_Eq.eqInt.eq(left)(right)) {
      return Data_Eq.eqInt.eq(right)(left);
    } else {
      return false;
    }
  };
}

export function compareArraysTwice(left) {
  return right => {
    const call = Data_Eq.eqArray(Data_Eq.eqInt);
    if (call.eq(left)(right)) {
      return call.eq(right)(left);
    } else {
      return false;
    }
  };
}

export function compareArraysOnce(left) {
  return right => {
    return (Data_Eq.eqArray(Data_Eq.eqInt)).eq(left)(right);
  };
}

export function compareGenericArraysTwice(eqADict) {
  function compareGenericArraysTwice$closure(eqADict) {
    return left => {
      return right => {
        const call = Data_Eq.eqArray(eqADict);
        if (call.eq(left)(right)) {
          return call.eq(right)(left);
        } else {
          return false;
        }
      };
    };
  }
  return compareGenericArraysTwice$closure(eqADict);
}

export function compareNestedArraysTwice(left) {
  return right => {
    return nestedLeft => {
      return nestedRight => {
        const call = Data_Eq.eqArray(Data_Eq.eqInt);
        const call$1 = Data_Eq.eqArray(call);
        if (call$1.eq(nestedLeft)(nestedRight)) {
          return call$1.eq(nestedRight)(nestedLeft);
        } else {
          return call.eq(left)(right);
        }
      };
    };
  };
}

export function distinctGivens(eqADict) {
  return eqBDict => {
    function distinctGivens$closure(eqADict, eqBDict) {
      return leftA => {
        return rightA => {
          return leftB => {
            return rightB => {
              function if$join$1(result$1) {
                return result$1;
              }

              const call = Data_Eq.eqArray(eqADict);
              const call$1 = Data_Eq.eqArray(eqBDict);
              if (call.eq(leftA)(rightA)) {
                return call.eq(rightA)(leftA);
              } else {
                if (call$1.eq(leftB)(rightB)) {
                  return if$join$1(call$1.eq(rightB)(leftB));
                } else {
                  return if$join$1(false);
                }
              }
            };
          };
        };
      };
    }
    return distinctGivens$closure(eqADict, eqBDict);
  };
}

export function distinctSubgoals(leftInt) {
  return rightInt => {
    return leftBoolean => {
      return rightBoolean => {
        function if$join$1(result$1) {
          return result$1;
        }

        const call = Data_Eq.eqArray(Data_Eq.eqInt);
        const call$1 = Data_Eq.eqArray(Data_Eq.eqBoolean);
        if (call.eq(leftInt)(rightInt)) {
          return call.eq(rightInt)(leftInt);
        } else {
          if (call$1.eq(leftBoolean)(rightBoolean)) {
            return if$join$1(call$1.eq(rightBoolean)(leftBoolean));
          } else {
            return if$join$1(false);
          }
        }
      };
    };
  };
}

export function compareArraysThrice(left) {
  return right => {
    function if$join$1(result$1) {
      return result$1;
    }

    const call = Data_Eq.eqArray(Data_Eq.eqInt);
    if (call.eq(left)(right)) {
      if (call.eq(right)(left)) {
        return if$join$1(call.eq(left)(right));
      } else {
        return if$join$1(false);
      }
    } else {
      return false;
    }
  };
}

export function compareNestedArraysWhole(left) {
  return right => {
    const call$1 = Data_Eq.eqArray(Data_Eq.eqArray(Data_Eq.eqInt));
    if (call$1.eq(left)(right)) {
      return call$1.eq(right)(left);
    } else {
      return false;
    }
  };
}

export function compareSuperclassArraysTwice(orderedADict) {
  function compareSuperclassArraysTwice$closure(orderedADict) {
    return left => {
      return right => {
        const call$1 = Data_Eq.eqArray(orderedADict.Eq0({}));
        if (call$1.eq(left)(right)) {
          return call$1.eq(right)(left);
        } else {
          return false;
        }
      };
    };
  }
  return compareSuperclassArraysTwice$closure(orderedADict);
}

export function compareSuperclassTwice(orderedADict) {
  function compareSuperclassTwice$closure(orderedADict) {
    return left => {
      return right => {
        const call = orderedADict.Eq0({});
        if (call.eq(left)(right)) {
          return call.eq(right)(left);
        } else {
          return false;
        }
      };
    };
  }
  return compareSuperclassTwice$closure(orderedADict);
}

export function lambdaScope(left) {
  return right => {
    if ((Data_Eq.eqArray(Data_Eq.eqInt)).eq(left)(right)) {
      function lambdaScope$closure(lambdaLeft) {
        return lambdaRight => {
          const call = Data_Eq.eqArray(Data_Eq.eqInt);
          if (call.eq(lambdaLeft)(lambdaRight)) {
            return call.eq(lambdaRight)(lambdaLeft);
          } else {
            return false;
          }
        };
      }
      return lambdaScope$closure(left)(right);
    } else {
      return false;
    }
  };
}

export function whereIsolation(left) {
  return right => {
    if ((helperLeft => helperRight => (Data_Eq.eqArray(Data_Eq.eqInt)).eq(helperLeft)(helperRight))(
      left
    )(right)) {
      return (Data_Eq.eqArray(Data_Eq.eqInt)).eq(left)(right);
    } else {
      return false;
    }
  };
}

export function equationScope($boolean) {
  return $array => {
    return $array$1 => {
      const call = Data_Eq.eqArray(Data_Eq.eqInt);
      if ($boolean === true) {
        return call.eq($array)($array$1);
      } else {
        if ($boolean === false) {
          return call.eq($array$1)($array);
        } else {
          throw new Error("Pattern match failure");
        }
      }
    };
  };
}

export const firstComparison = Data_Eq.eqInt.eq(1 | 0)(2 | 0);

export const secondComparison = Data_Eq.eqInt.eq(3 | 0)(4 | 0);
