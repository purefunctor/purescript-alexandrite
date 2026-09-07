module Main where

data Choice a = Empty | One a | Pair a a

guarded condition choice =
  case choice of
    One nested
      | condition, One value <- nested -> value
    Pair left right
      | One value <- left -> value
      | One value <- right -> value
    _ -> 0

withWhere choice =
  case choice of
    One value -> local
      where
      local = value
    _ -> 0
