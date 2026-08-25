export function name(dictionary) {
  return dictionary.name;
}

export function use(namedADict) {
  return value => name(namedADict)(value);
}

export function useLet(namedADict) {
  return value => name(namedADict)(value);
}

export const namedString = (() => {
  return { name: value => value };
})();
