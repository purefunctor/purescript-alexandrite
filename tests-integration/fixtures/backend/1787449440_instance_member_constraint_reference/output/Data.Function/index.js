export function on(operation) {
  return projection => {
    return left => {
      return right => {
        return operation(projection(left))(projection(right));
      };
    };
  };
}
