module Main where

foreign import data Box :: Type -> Type

foreign import polymorphic :: forall value. Box value

foreign import bind ::
  forall result.
  (forall value. Box value) ->
  ((forall value. Box value) -> result) ->
  result

higherRank :: Int
higherRank = do
  value <- polymorphic
  42
