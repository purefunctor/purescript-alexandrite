module Main where

class Negative :: Type -> Constraint
class Negative a

foreign import negate :: forall a. Negative a => a -> a

negative :: forall a. Negative a => a -> a
negative value = -value

shadowedNegate :: (Int -> Int) -> Int -> Int
shadowedNegate negate value = -value
