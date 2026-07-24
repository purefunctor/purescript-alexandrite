module Main where

foreign import data Effect :: Type -> Type

foreign import pure :: forall value. value -> Effect value
foreign import bind :: forall value result. Effect value -> (value -> Effect result) -> Effect result
foreign import discard :: forall value result. Effect value -> (value -> Effect result) -> Effect result

emptyDo = do

finalBind = do
  value <- pure 1

finalLet = do
  let value = 1

missingDoAction = do
  value <-
  pure value
