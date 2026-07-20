module Main where

data Choice a = Empty | One a | Pair a a

newtype Wrapped a = Wrapped (Choice a)

data Fix :: forall k. ((k -> Type) -> k -> Type) -> k -> Type
data Fix f a = Fix (f (Fix f) a)

infixr 5 Pair as :+:

bare = One

nullary = Empty

applied = One 1

partial = Pair 1

operatorApplied = 1 :+: 2

operatorBare = (:+:)

unwrapOne (One value) = value

unwrapPair (left :+: right) = left

unwrapNested (first :+: second :+: third) = first

choose choice = case choice of
  Empty -> 0
  One value -> value
  Pair left _ -> left

guarded condition choice = case choice of
  One nested
    | condition, One value <- nested -> value
  Pair left right
    | One value <- left -> value
    | One value <- right -> value
  _ -> 0

matchPartial choice = case choice of
  One value -> value

chooseNested choice = case choice of
  One nested -> case nested of
    One value -> value
    Pair left _ -> left
    _ -> 0
  _ -> 0

wrapped = Wrapped Empty
