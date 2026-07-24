module Main where

foreign import data Action :: Int -> Type -> Type

class OnStep :: Int -> Constraint
class OnStep step

instance onZero :: OnStep 0
else instance onTwo :: OnStep 2
else instance onDefault :: OnStep step

foreign import map ::
  forall step.
  OnStep step =>
  forall value result.
  (value -> result) ->
  Action step value ->
  Action step result

foreign import apply ::
  forall step.
  OnStep step =>
  forall previous value result.
  Action previous (value -> result) ->
  Action step value ->
  Action step result

foreign import pure ::
  forall step.
  OnStep step =>
  forall value.
  value ->
  Action step value

foreign import stepZero :: Action 0 Int

foreign import stepOne :: Action 1 String

foreign import stepTwo :: Action 2 Boolean

foreign import stepThree :: Action 3 String

indexedApply :: Action 3 String
indexedApply = ado
  zero <- stepZero
  one <- stepOne
  two <- stepTwo
  three <- stepThree
  in three

indexedPure :: Action 2 Int
indexedPure = ado
  in 1
