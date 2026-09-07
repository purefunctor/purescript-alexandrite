module Main where

data Proxy :: forall kind. kind -> Type
data Proxy value = Proxy

class Dependent :: forall kind. kind -> Constraint
class Dependent value where
  dependent :: Proxy value

instance Dependent value where
  dependent :: Proxy value
  dependent = Proxy :: Proxy value
