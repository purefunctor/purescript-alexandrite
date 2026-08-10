module Main where

class Value :: Type -> Constraint
class Value a

constrainedIdentity :: forall a. Value a => a -> a
constrainedIdentity value = value

test :: forall a. Value a => a -> a
test = (constrainedIdentity :: forall b. Value b => b -> b)
