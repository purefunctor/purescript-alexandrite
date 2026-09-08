module Main where

foreign import data Box :: Type -> Type

foreign import polymorphic :: forall value. Box value

foreign import bind ::
  forall result.
  (forall value. Box value) ->
  ((forall value. Box value) -> result) ->
  result

test :: Int
test = do
  value <- polymorphic
  42

test' = do
  value <- polymorphic
  42
