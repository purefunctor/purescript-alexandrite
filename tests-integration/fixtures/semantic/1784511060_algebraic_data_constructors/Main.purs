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

wrapped = Wrapped Empty
