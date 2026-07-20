module IndexedAdo where

import Prim.Int (class Add)

foreign import data Render :: Int -> Int -> Type -> Type

data Unit = Unit

foreign import unsafeCoerce :: forall a b. a -> b

class MapAt (state :: Int) where
  map ::
    forall a b next.
    Add state 1 next =>
    (a -> b) ->
    Render state state a ->
    Render state next b

class ApplyAt (state :: Int) where
  apply ::
    forall a b start next.
    Add state 1 next =>
    Render start state (a -> b) ->
    Render state state a ->
    Render start next b

instance mapAtZero :: MapAt 0 where
  map = unsafeCoerce

else instance mapAtFallback :: MapAt state where
  map = unsafeCoerce

instance applyAtTwo :: ApplyAt 2 where
  apply = unsafeCoerce

else instance applyAtFallback :: ApplyAt state where
  apply = unsafeCoerce

pure :: forall a state. a -> Render state state a
pure = unsafeCoerce

action :: forall state. Render state state Unit
action = unsafeCoerce Unit
