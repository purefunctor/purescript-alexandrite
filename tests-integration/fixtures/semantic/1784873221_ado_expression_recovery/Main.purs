module Main where

foreign import data Effect :: Type -> Type

foreign import pure :: forall value. value -> Effect value
foreign import map :: forall value result. (value -> result) -> Effect value -> Effect result
foreign import apply :: forall value result. Effect (value -> result) -> Effect value -> Effect result

emptyAdo = ado

missingAdoAction = ado
  value <-
  in value

missingAdoResult = ado
  value <- pure 1
  in
