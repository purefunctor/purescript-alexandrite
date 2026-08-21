export const Wrapper = $value0 => ["Wrapper", $value0];

export function genericEqual(dictionary0) {
  function genericEqual$closure(dictionary0) {
    return left => {
      return right => {
        return dictionary0.equal(left)(right);
      };
    };
  }
  return genericEqual$closure(dictionary0);
}

export function arrayEqual(dictionary1) {
  function arrayEqual$closure(dictionary1) {
    return left => {
      return right => {
        return dictionary1.equal(left)(right);
      };
    };
  }
  return arrayEqual$closure(dictionary1);
}

export function wrapperEqual(dictionary2) {
  function wrapperEqual$closure(dictionary2) {
    return left => {
      return right => {
        return dictionary2.equal(left)(right);
      };
    };
  }
  return wrapperEqual$closure(dictionary2);
}

export function concreteEqual(dictionary3) {
  function concreteEqual$closure(dictionary3) {
    return left => {
      return right => {
        return dictionary3.equal(left)(right);
      };
    };
  }
  return concreteEqual$closure(dictionary3);
}

export function convertToInt(dictionary4) {
  function convertToInt$closure(dictionary4) {
    return value => {
      return dictionary4.convert(value);
    };
  }
  return convertToInt$closure(dictionary4);
}

export function distinctEqual(dictionary5) {
  return dictionary6 => {
    function distinctEqual$closure(dictionary5, dictionary6) {
      return left1 => {
        return left2 => {
          return right1 => {
            return right2 => {
              return {
                left: dictionary5.equal(left1)(left2),
                right: dictionary6.equal(right1)(right2)
              };
            };
          };
        };
      };
    }
    return distinctEqual$closure(dictionary5, dictionary6);
  };
}

export function duplicateEqual(dictionary7) {
  return dictionary8 => {
    function duplicateEqual$closure(dictionary8) {
      return left => {
        return right => {
          return dictionary8.equal(left)(right);
        };
      };
    }
    return duplicateEqual$closure(dictionary8);
  };
}

export function parameterCollision(dictionary9) {
  function parameterCollision$closure(dictionary9) {
    return equalValueDict => {
      return left => {
        return right => {
          if (equalValueDict) {
            return dictionary9.equal(left)(right);
          } else {
            return false;
          }
        };
      };
    };
  }
  return parameterCollision$closure(dictionary9);
}

export function isAvailable(dictionary10) {
  return dictionary10.available;
}
