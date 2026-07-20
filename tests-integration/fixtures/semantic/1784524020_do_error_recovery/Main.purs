module Main where

import PartialDo as Partial

foreign import data Box :: Type -> Type

foreign import bind :: forall a b. Box a -> (a -> Box b) -> Box b
foreign import discard :: forall a b. Box a -> (a -> Box b) -> Box b
foreign import pure :: forall a. a -> Box a

foreign import boxedInt :: Box Int

missingAction :: Box Int
missingAction = do
  value <-
  pure 42

missingFinalAction :: Box Int
missingFinalAction = do
  value <- boxedInt
  result <-

localLet :: Box Int
localLet = do
  value <- boxedInt
  let kept = value
  pure kept

finalBind :: Box Int
finalBind = do
  value <- pure 42

finalLet :: Box Int
finalLet = do
  let value = 42

empty = do

partialApplication = Partial.do
  value <- Partial.boxedInt
  Partial.pure value
