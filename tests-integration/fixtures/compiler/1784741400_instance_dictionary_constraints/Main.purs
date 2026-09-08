module Main where

class First :: Type -> Constraint
class First a

class Second :: Type -> Constraint
class Second b

class Example a b where
  example :: a -> b -> Boolean

implementation :: forall b a. Second b => a -> b -> Boolean
implementation _ _ = boolean

foreign import boolean :: Boolean

instance exampleAB :: (First a, Second b) => Example a b where
  example = implementation
