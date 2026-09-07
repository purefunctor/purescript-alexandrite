import * as $stylex from "@stylexjs/stylex";
export const variables = $stylex.defineVars({
  accent: $stylex.types.color("blue"),
  spacing: $stylex.types.length("8px")
});
export const valid = $stylex.createTheme(variables, { accent: $stylex.types.color("white") });
export const wrongType = $stylex.createTheme(variables, { accent: $stylex.types.length("12px") });
export const unknown = $stylex.createTheme(variables, { missing: $stylex.types.color("red") });
export const invalidInteger = $stylex.defineVars({ value: $stylex.types.integer(true) });
export const invalidNumber = $stylex.defineVars({ value: $stylex.types.number("1") });
export const invalidColor = $stylex.defineVars({ value: $stylex.types.color(42 | 0) });
