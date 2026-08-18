module Main where

class Equal a where
  equal :: a -> a -> Boolean

class Equal a <= Ordered a where
  lessThan :: a -> a -> Boolean

foreign import equalInt :: Int -> Int -> Boolean
foreign import lessThanInt :: Int -> Int -> Boolean

instance Equal Int where
  equal = equalInt

instance Ordered Int where
  lessThan = lessThanInt

genericEqual :: forall a. Equal a => a -> a -> Boolean
genericEqual left right = equal left right

concreteEqual :: Boolean
concreteEqual = equal 1 2

concreteLessThan :: Boolean
concreteLessThan = lessThan 1 2

superclassEqual :: forall a. Ordered a => a -> a -> Boolean
superclassEqual left right = equal left right
