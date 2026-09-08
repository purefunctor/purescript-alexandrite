module Main where

foreign import data Action :: Int -> Type -> Type

class OnStep :: Int -> Constraint
class OnStep step

instance onZero :: OnStep 0
else instance onTwo :: OnStep 2
else instance onDefault :: OnStep step

foreign import bind ::
  forall step.
  OnStep step =>
  forall next value result.
  Action step value ->
  (value -> Action next result) ->
  Action step result

foreign import discard ::
  forall step.
  OnStep step =>
  forall next value result.
  Action step value ->
  (value -> Action next result) ->
  Action step result

foreign import stepZero :: Action 0 Int

foreign import stepOne :: Action 1 String

foreign import stepTwo :: Action 2 Boolean

foreign import stepThree :: Action 3 String

foreign import stepFour :: Action 4 Number

indexedBind :: Action 0 Number
indexedBind = do
  zero <- stepZero
  one <- stepOne
  two <- stepTwo
  three <- stepThree
  stepFour

indexedDiscard :: Action 4 Boolean
indexedDiscard = do
  stepFour
  stepTwo
