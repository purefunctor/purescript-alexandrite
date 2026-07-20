module IndexedAdo where

foreign import data Render :: Type -> Type -> Type -> Type

data Unit = Unit

foreign import data Start :: Type

foreign import data Use1 :: Type -> Type
foreign import data Use2 :: Type -> Type

foreign import map ::
  forall a b x y.
  (a -> b) ->
  Render x y a ->
  Render x y b

foreign import apply ::
  forall a b x y z.
  Render x y (a -> b) ->
  Render y z a ->
  Render x z b

foreign import pure :: forall a x. a -> Render x x a

foreign import use1 :: forall hooks. Render hooks (Use1 hooks) Unit
foreign import use2 :: forall hooks. Render hooks (Use2 hooks) Unit
