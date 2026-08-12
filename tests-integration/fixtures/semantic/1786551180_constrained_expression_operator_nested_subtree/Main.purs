module Main where

class First a

foreign import apply
  :: forall argument result
   . (argument -> result)
  -> argument
  -> result

infixr 0 apply as $

foreign import identity :: forall a. a -> a
foreign import constrainedValue :: First Int => Int
foreign import consumeConstrained :: (First Int => Int) -> Int

constrainedSubtree :: Int
constrainedSubtree = consumeConstrained $ identity $ constrainedValue
