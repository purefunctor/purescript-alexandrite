module Main where

data Choice a = Empty | One a | Pair a a

checked :: Choice Int -> Int
checked choice =
  case choice of
    Empty -> 0
    One value -> value
    Pair left _ -> left

inferred choice =
  case choice of
    Empty -> 0
    One value -> value
    Pair left _ -> left

multiple first second =
  case first, second of
    One left, One _ -> left
    Pair left _, _ -> left
    _, One right -> right
    _, _ -> 0

partial choice =
  case choice of
    One value -> value
