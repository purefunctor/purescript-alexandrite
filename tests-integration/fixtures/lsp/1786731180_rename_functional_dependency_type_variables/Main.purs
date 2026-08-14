module Main where

foreign import data Proxy :: forall kind. kind -> Type

class TypeEq :: forall kind. kind -> kind -> Boolean -> Constraint
class TypeEq a b result | a b -> result where
--                        /
  compare :: Proxy a -> Proxy b -> Proxy result
