module Data.Eq (class Eq, eq, eqInt) where

class Eq a where
  eq :: a -> a -> Boolean

instance eqInt :: Eq Int where
  eq _ _ = true
