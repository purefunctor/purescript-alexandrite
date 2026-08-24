export const mkFn2 = function (function_) {
  return function (first, second) {
    return function_(first)(second);
  };
};

export const mkFn3 = function (function_) {
  return function (first, second, third) {
    return function_(first)(second)(third);
  };
};

export const runFn2 = function (function_) {
  return function (first) {
    return function (second) {
      return function_(first, second);
    };
  };
};

export const runFn3 = function (function_) {
  return function (first) {
    return function (second) {
      return function (third) {
        return function_(first, second, third);
      };
    };
  };
};
