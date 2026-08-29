import * as $stylex from "@stylexjs/stylex";
export const animation = $stylex.keyframes({
  from: { opacity: 0 },
  to: { opacity: 1 }
});
export const styles = $stylex.create({
  button: {
    color: "red",
    padding: 8 | 0,
    animationName: animation,
    ":hover": { color: "blue" }
  },
  label: { fontWeight: 600 | 0 }
});
export const secondary = $stylex.create({ root: { backgroundColor: "navy" } });
export const buttonProps = $stylex.props(styles.button);
export const buttonClassName = buttonProps.className;
