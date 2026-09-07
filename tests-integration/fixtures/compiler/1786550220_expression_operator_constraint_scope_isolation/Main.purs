module Main where

class First a

foreign import combine
  :: forall result
   . (First Int => Int)
  -> result
  -> result

infixr 0 combine as <+>

foreign import constrainedValue :: First Int => Int
foreign import needsFirst :: First Int => Int

test :: Int
test = constrainedValue <+> needsFirst
