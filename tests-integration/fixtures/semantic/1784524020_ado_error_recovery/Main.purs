module Main where

import ErrorAdo as Error
import ErrorApplyAdo as ErrorApply
import PartialAdo as Partial
import PartialApplyAdo as PartialApply

foreign import data Box :: Type -> Type

data Pair a b = Pair a b

foreign import map :: forall a b. (a -> b) -> Box a -> Box b
foreign import apply :: forall a b. Box (a -> b) -> Box a -> Box b
foreign import pure :: forall a. a -> Box a

foreign import boxedInt :: Box Int
foreign import boxedString :: Box String

missingAction :: Box (Pair Int String)
missingAction = ado
  left <-
  right <- boxedString
  in Pair left right

soleMissingAction :: Box Int
soleMissingAction = ado
  value <-
  in value

localLet :: Box (Pair Int String)
localLet = ado
  left <- boxedInt
  let kept = left
  right <- boxedString
  in Pair kept right

missingIn :: Box Int
missingIn = ado
  value <- boxedInt
  in

pureLet :: Box Int
pureLet = ado
  let value = 42
  in value

empty = ado

partialApplication = Partial.ado
  value <- Partial.boxedInt
  in value

errorApplication = Error.ado
  value <- Error.boxedInt
  in value

partialApplyApplication = PartialApply.ado
  first <- PartialApply.boxedInt
  second <- PartialApply.boxedInt
  in Pair first second

errorApplyApplication = ErrorApply.ado
  first <- ErrorApply.boxedInt
  second <- ErrorApply.boxedInt
  in Pair first second

errorPureApplication = Partial.ado in 42
