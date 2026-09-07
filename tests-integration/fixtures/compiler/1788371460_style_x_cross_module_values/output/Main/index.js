import { rowMarker as Tokens_rowMarker, variables as Tokens_variables } from "../Tokens/index.js";
import * as $stylex from "@stylexjs/stylex";
export const theme = $stylex.createTheme(Tokens_variables, { accent: "white" });
export const styles = $stylex.create({ root: { color: {
  default: "blue",
  [$stylex.when.ancestor(":hover", Tokens_rowMarker)]: "red"
} } });
