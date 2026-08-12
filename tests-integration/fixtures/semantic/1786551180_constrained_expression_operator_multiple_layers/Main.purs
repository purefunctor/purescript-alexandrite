module Main where

class First a
class Second a

foreign import apply
  :: forall argument result
   . (argument -> result)
  -> argument
  -> result

infixr 0 apply as $

foreign import multiplyConstrained :: First Int => Second Int => Int
foreign import consumeMultiplyConstrained :: (First Int => Second Int => Int) -> Int

multipleConstraintLayers :: Int
multipleConstraintLayers = consumeMultiplyConstrained $ multiplyConstrained
