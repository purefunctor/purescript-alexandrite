module Data.Eq where

class Eq a where
  eq :: a -> a -> Boolean

instance Eq Int where
  eq _ _ = true

instance Eq Boolean where
  eq _ _ = true

instance eqArray :: Eq a => Eq (Array a) where
  eq _ _ = true

class Eq a <= Ordered a where
  lessThanOrEqual :: a -> a -> Boolean

instance Ordered Int where
  lessThanOrEqual _ _ = true
