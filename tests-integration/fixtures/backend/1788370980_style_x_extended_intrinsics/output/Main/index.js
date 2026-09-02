import * as $stylex from "@stylexjs/stylex";
export const constants = $stylex.defineConsts({
  compact: "@media (max-width: 40rem)",
  columns: 12 | 0
});
export const variables = $stylex.defineVars({
  accent: $stylex.types.color("royalblue"),
  angle: $stylex.types.angle("45deg"),
  image: $stylex.types.image("linear-gradient(red, blue)"),
  integer: $stylex.types.integer(1 | 0),
  length: $stylex.types.length("8px"),
  lengthPercentage: $stylex.types.lengthPercentage("10%"),
  number: $stylex.types.number(.5),
  percentage: $stylex.types.percentage("50%"),
  resolution: $stylex.types.resolution("2dppx"),
  time: $stylex.types.time("200ms"),
  transformFunction: $stylex.types.transformFunction("scale(1)"),
  transformList: $stylex.types.transformList("scale(1) rotate(2deg)"),
  url: $stylex.types.url("url(image.png)")
});
export const theme = $stylex.createTheme(variables, {
  accent: $stylex.types.color("white"),
  length: $stylex.types.length("12px")
});
export const rowMarker = $stylex.defineMarker();
export const styles = $stylex.create({ root: {
  color: {
    default: "blue",
    [$stylex.when.ancestor(":hover")]: "red",
    [$stylex.when.ancestor(":focus", rowMarker)]: "green",
    [$stylex.when.descendant(":hover")]: "purple",
    [$stylex.when.descendant(":focus", rowMarker)]: "pink",
    [$stylex.when.siblingBefore(":hover")]: "orange",
    [$stylex.when.siblingBefore(":focus", rowMarker)]: "yellow",
    [$stylex.when.siblingAfter(":hover")]: "gray",
    [$stylex.when.siblingAfter(":focus", rowMarker)]: "black",
    [$stylex.when.anySibling(":hover")]: "navy",
    [$stylex.when.anySibling(":focus", rowMarker)]: "teal"
  },
  position: $stylex.firstThatWorks("sticky", "fixed")
} });
export const attributes = $stylex.attrs(styles.root);
export const markerProps = $stylex.props(rowMarker);
export const defaultMarkerProps = $stylex.props($stylex.defaultMarker());
export const transitionClass = $stylex.viewTransitionClass({
  new: { opacity: 1 },
  old: { opacity: 0 }
});
export const fallback = $stylex.positionTry({
  positionArea: "block-start",
  margin: 8 | 0
});
