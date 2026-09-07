module Main where

import Prim.RowList (class RowToList)

foreign import unsafeCoerce :: forall a b. a -> b

data M (context :: Row Type) a = M a

bind :: forall context a b. M context a -> (a -> M context b) -> M context b
bind (M a) f = f a

class Ask (context :: Row Type) (m :: Type -> Type) | m -> context where
  ask :: m (Record context)

instance Ask context (M context) where
  ask = unsafeCoerce (M {})

consume :: forall row list. RowToList row list => Record row -> {}
consume _ = {}

class Produce (context :: Row Type) (input :: Row Type) (output :: Row Type) | context input -> output where
  produce :: Record context -> Record input -> Record output

instance RowToList input inputList => Produce context input input where
  produce _ input = input

test
  :: forall context input output outputList
   . RowToList output outputList
  => Produce context input output
  => Record input
  -> M context {}
test input =
  bind ask \context ->
    let output = produce context input
    in M (consume output)
