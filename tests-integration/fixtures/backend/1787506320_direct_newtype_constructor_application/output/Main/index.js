export function firstClass(value) {
  return value;
}

export const direct = (() => {
  function direct$initialize$closure(value) {
    return value;
  }
  return direct$initialize$closure(42 | 0);
})();

export const indirect = firstClass(43 | 0);
