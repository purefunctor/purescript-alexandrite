module Main where

foreign import data Effect :: Type -> Type

foreign import bind :: forall a b. Effect a -> (a -> Effect b) -> Effect b
foreign import before :: Effect { field :: Int }
foreign import expectString :: String -> Int

test = do
  record <- before
  let _ = expectString record.field
