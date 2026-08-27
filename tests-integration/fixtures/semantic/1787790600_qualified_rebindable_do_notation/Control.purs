module Control where

foreign import data Box :: Type -> Type

foreign import action :: forall value. Box value

foreign import bind ::
  forall value result.
  Box value ->
  (value -> Box result) ->
  Box result

foreign import discard ::
  forall value result.
  Box value ->
  (value -> Box result) ->
  Box result

foreign import map ::
  forall value result.
  (value -> result) ->
  Box value ->
  Box result

foreign import apply ::
  forall value result.
  Box (value -> result) ->
  Box value ->
  Box result

foreign import pure :: forall value. value -> Box value
