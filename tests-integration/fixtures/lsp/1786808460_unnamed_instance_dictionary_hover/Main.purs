module Main where

data Pair a b = Pair a b

infixr 6 type Pair as :+:

class Convert :: Type -> Type -> Constraint
class Convert source target

instance Convert Int String
-- $

instance Convert (Int :+: String) Boolean
-- $

instance Convert value value
-- $

instance Convert value (value :+: value)
-- $
