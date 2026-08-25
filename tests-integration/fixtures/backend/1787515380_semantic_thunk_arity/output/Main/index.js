export const Unit = "Unit";

export function ordinaryUnit(value) {
  return value;
}

export function parent(dictionary) {
  return dictionary.parent;
}

export function child(dictionary) {
  return dictionary.child;
}

export function childArray(parentArrayADict) {
  return { Parent0: () => parentArrayADict, child: value => value };
}

export function useSuperclass(childADict) {
  return value => parent(childADict.Parent0())(value);
}

export const ordinaryUnitCall = ordinaryUnit(Unit);

export const parentInt = { parent: value => value };
