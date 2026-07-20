module Main where

class Combine a where
  combine :: a -> a -> a

instance combineInt :: Combine Int where
  combine = combineIntImpl

foreign import combineIntImpl :: Int -> Int -> Int

infixl 5 combine as <>

resolved :: Int
resolved = 1 <> 2

given :: forall a. Combine a => a -> a -> a
given left right = left <> right
