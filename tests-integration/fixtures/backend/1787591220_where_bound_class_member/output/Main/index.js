export function name(dictionary) {
  return dictionary.name;
}

export function use(namedADict) {
  const $closure = value => {
    const alias = name;
    return alias(namedADict)(value);
  };
  return $closure;
}

export function useLet(namedADict) {
  const $closure = value => {
    const alias = name;
    return alias(namedADict)(value);
  };
  return $closure;
}

export const namedString = { name: value => value };
