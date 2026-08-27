export const mkFn2 = function_ => (first, second) => function_(first)(second);
export const runFn2 = function_ => first => second => function_(first, second);
