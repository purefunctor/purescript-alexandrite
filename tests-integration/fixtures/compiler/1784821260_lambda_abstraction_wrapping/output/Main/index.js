export function identity(value) {
  return value;
}
export function wrap(firstArgument) {
  return (secondArgument) => {
    return (thirdArgument) => {
      return (fourthArgument) => {
        return (fifthArgument) => {
          return (sixthArgument) => {
            return firstArgument;
          };
        };
      };
    };
  };
}
