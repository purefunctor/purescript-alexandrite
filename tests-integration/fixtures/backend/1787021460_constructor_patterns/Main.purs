module Main where

data Choice a = Empty | One a | Pair a a

newtype Identity a = Identity a

first :: forall a. Choice a -> Choice a
first Empty = Empty
first (One value) = One value
first whole@(Pair left _) = case whole of
  Pair _ _ -> One left
  _ -> Empty

unwrap :: forall a. Identity a -> a
unwrap (Identity value) = value
