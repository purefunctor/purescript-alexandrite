module Data.Eq where

class Eq a where
  eq :: a -> a -> Boolean

instance Eq Int where
  eq _ _ = true

instance eqArray :: Eq a => Eq (Array a) where
  eq _ _ = true
