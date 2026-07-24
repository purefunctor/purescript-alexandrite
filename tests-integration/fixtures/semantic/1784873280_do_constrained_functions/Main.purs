module Main where

class Operations :: (Type -> Type) -> Constraint
class Operations effect where
  pure :: forall value. value -> effect value
  bind :: forall value result. effect value -> (value -> effect result) -> effect result
  discard :: forall value result. effect value -> (value -> effect result) -> effect result

usingBind :: forall effect. Operations effect => effect Int
usingBind = do
  value <- pure 1
  pure value

usingDiscard :: forall effect. Operations effect => effect Int
usingDiscard = do
  pure "ignored"
  pure 1
