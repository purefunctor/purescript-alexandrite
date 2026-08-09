module Main where

identity :: forall a. a -> a
identity value = value

consume :: (forall a. a -> a) -> Int
consume _ = 0

higherRank :: Int
higherRank = consume (identity :: forall a. a -> a)
