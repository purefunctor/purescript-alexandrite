module Main where

data Choice a = Empty | One a | Pair a a

identity :: forall a. a -> a
identity value = value

nested choice =
  case choice of
    One inner ->
      case inner of
        One value -> value
        Pair left _ -> left
        _ -> 0
    _ -> 0

applied choice = identity case choice of
  One value -> value
  _ -> 0
