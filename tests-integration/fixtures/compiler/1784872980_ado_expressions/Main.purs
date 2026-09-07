module Main where

foreign import data Effect :: Type -> Type

data Tuple first second = Tuple first second

foreign import pure :: forall value. value -> Effect value

foreign import map ::
  forall value result.
  (value -> result) ->
  Effect value ->
  Effect result

foreign import apply ::
  forall value result.
  Effect (value -> result) ->
  Effect value ->
  Effect result

zero = ado
  in 1

zeroWithLet = ado
  let value = 1
  in value

one = ado
  value <- pure 1
  in value

multiple = ado
  first <- pure 1
  second <- pure "two"
  in Tuple first second

discarded = ado
  pure 1
  value <- pure "kept"
  in value

withLet = ado
  first <- pure 1
  let next = first
  second <- pure 2
  in Tuple next second

nested = ado
  value <- ado
    inner <- pure 1
    in inner
  in value
