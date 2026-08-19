function showInt$initialize$closure(argument0) {
  const literal = "";
  return literal;
}

function showArray$closure(argument0) {
  const literal = "";
  return literal;
}

export function showArray(dictionary0) {
  const closure = showArray$closure;
  const record = { show: closure };
  return record;
}

export const showInt = { show: showInt$initialize$closure };
