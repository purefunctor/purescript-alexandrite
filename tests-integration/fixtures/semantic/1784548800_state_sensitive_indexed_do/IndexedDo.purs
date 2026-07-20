module IndexedDo where

import Prim.Int (class Add)

foreign import data Render :: Int -> Int -> Type -> Type

data Unit = Unit

foreign import unsafeCoerce :: forall a b. a -> b

class BindAt (state :: Int) where
  bind ::
    forall a b next end.
    Add state 1 next =>
    Render state state a ->
    (a -> Render next end b) ->
    Render state end b
  discard ::
    forall a b next end.
    Add state 1 next =>
    Render state state a ->
    (a -> Render next end b) ->
    Render state end b

instance bindAtZero :: BindAt 0 where
  bind = unsafeCoerce
  discard = unsafeCoerce

else instance bindAtTwo :: BindAt 2 where
  bind = unsafeCoerce
  discard = unsafeCoerce

else instance bindAtFallback :: BindAt state where
  bind = unsafeCoerce
  discard = unsafeCoerce

pure :: forall a state. a -> Render state state a
pure = unsafeCoerce

action :: forall state. Render state state Unit
action = unsafeCoerce Unit
