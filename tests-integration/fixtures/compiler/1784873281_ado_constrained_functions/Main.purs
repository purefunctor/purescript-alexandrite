module Main where

data Tuple first second = Tuple first second

class Operations :: (Type -> Type) -> Constraint
class Operations effect where
  pure :: forall value. value -> effect value
  map :: forall value result. (value -> result) -> effect value -> effect result
  apply :: forall value result. effect (value -> result) -> effect value -> effect result

usingMapApply :: forall effect. Operations effect => effect (Tuple Int String)
usingMapApply = ado
  first <- pure 1
  second <- pure "two"
  in Tuple first second

usingPure :: forall effect. Operations effect => effect Int
usingPure = ado
  in 1
