export const Unit = "Unit";
export const Yes = "Yes";
export const No = "No";
export function unit($unit) {
  if ($unit === "Unit") {
    return 1 | 0;
  }
  return 2 | 0;
}
export function yes($yesNo) {
  if ($yesNo === "Yes") {
    return 1 | 0;
  }
  return 2 | 0;
}
export function no($yesNo) {
  if ($yesNo === "Yes") {
    return 1 | 0;
  }
  return 2 | 0;
}
export function yesNo($yesNo) {
  if ($yesNo === "Yes") {
    return 1 | 0;
  }
  if ($yesNo === "No") {
    return 2 | 0;
  }
  return 3 | 0;
}
