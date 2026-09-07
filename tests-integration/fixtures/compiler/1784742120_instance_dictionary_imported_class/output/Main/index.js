export const Token = "Token";
export function childToken(parentTokenDict) {
  return {
    Parent0: () => parentTokenDict,
    child: "Token"
  };
}
