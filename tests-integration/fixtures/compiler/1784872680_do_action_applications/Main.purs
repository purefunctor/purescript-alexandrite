module Main where

foreign import data Effect :: Type -> Type

class Select value

foreign import constrained :: forall value. Select value => Effect value

foreign import pure :: forall value. value -> Effect value

foreign import bind ::
  forall value result.
  Effect value ->
  (value -> Effect result) ->
  Effect result

selected :: Select Int => Effect Int
selected = do
  value <- constrained
  pure value
