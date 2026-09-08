module Main where

class Required :: Type -> Constraint
class Required value

class Example a where
  apply :: forall b. Required b => a -> b -> Boolean

implementation :: forall a b. Required b => a -> b -> Boolean
implementation _ _ = boolean

foreign import boolean :: Boolean

instance Example Int where
  apply = implementation
