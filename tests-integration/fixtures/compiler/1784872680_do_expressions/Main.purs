module Main where

foreign import data Effect :: Type -> Type

foreign import pure :: forall value. value -> Effect value

foreign import bind ::
  forall value result.
  Effect value ->
  (value -> Effect result) ->
  Effect result

foreign import discard ::
  forall value result.
  Effect value ->
  (value -> Effect result) ->
  Effect result

bound :: Effect Int
bound = do
  value <- pure 1
  pure value

discarded = do
  pure 1
  pure "done"

withLet = do
  value <- pure 1
  let next = value
  pure next

nested = do
  value <- do
    inner <- pure 1
    pure inner
  pure value
