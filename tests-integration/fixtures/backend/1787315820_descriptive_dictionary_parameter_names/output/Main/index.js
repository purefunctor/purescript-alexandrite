export const Wrapper = $value0 => ["Wrapper", $value0];

export function genericEqual(equalValueDict) {
  function genericEqual$closure(equalValueDict) {
    return left => {
      return right => {
        return equalValueDict.equal(left)(right);
      };
    };
  }
  return genericEqual$closure(equalValueDict);
}

export function arrayEqual(equalArrayValueDict) {
  function arrayEqual$closure(equalArrayValueDict) {
    return left => {
      return right => {
        return equalArrayValueDict.equal(left)(right);
      };
    };
  }
  return arrayEqual$closure(equalArrayValueDict);
}

export function wrapperEqual(equalWrapperValueDict) {
  function wrapperEqual$closure(equalWrapperValueDict) {
    return left => {
      return right => {
        return equalWrapperValueDict.equal(left)(right);
      };
    };
  }
  return wrapperEqual$closure(equalWrapperValueDict);
}

export function concreteEqual(equalIntDict) {
  function concreteEqual$closure(equalIntDict) {
    return left => {
      return right => {
        return equalIntDict.equal(left)(right);
      };
    };
  }
  return concreteEqual$closure(equalIntDict);
}

export function convertToInt(convertValueIntDict) {
  function convertToInt$closure(convertValueIntDict) {
    return value => {
      return convertValueIntDict.convert(value);
    };
  }
  return convertToInt$closure(convertValueIntDict);
}

export function distinctEqual(equalLeftDict) {
  return equalRightDict => {
    function distinctEqual$closure(equalLeftDict, equalRightDict) {
      return left1 => {
        return left2 => {
          return right1 => {
            return right2 => {
              return {
                left: equalLeftDict.equal(left1)(left2),
                right: equalRightDict.equal(right1)(right2)
              };
            };
          };
        };
      };
    }
    return distinctEqual$closure(equalLeftDict, equalRightDict);
  };
}

export function duplicateEqual(equalValueDict) {
  return equalValueDict$1 => {
    function duplicateEqual$closure(equalValueDict) {
      return left => {
        return right => {
          return equalValueDict.equal(left)(right);
        };
      };
    }
    return duplicateEqual$closure(equalValueDict$1);
  };
}

export function parameterCollision(equalValueDict) {
  function parameterCollision$closure(equalValueDict) {
    return equalValueDict$1 => {
      return left => {
        return right => {
          if (equalValueDict$1) {
            return equalValueDict.equal(left)(right);
          } else {
            return false;
          }
        };
      };
    };
  }
  return parameterCollision$closure(equalValueDict);
}

export function isAvailable(availableDict) {
  return availableDict.available;
}
