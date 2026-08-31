export function matchesLead($string) {
  if ($string === "\ud800") {
    return true;
  }
  return false;
}
export const lead = "\ud800";
export const trail = "\udfff";
export const escapedPair = "𐀀";
export const scalar = "𐀀";
