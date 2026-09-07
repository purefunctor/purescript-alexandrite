module Main where

class Eq :: Type -> Constraint
class Eq a

class Eq a <= Ord a where
  compare :: a -> a -> Int

foreign import compareImpl :: Int -> Int -> Int

instance Eq Int => Ord Int where
  compare = compareImpl
