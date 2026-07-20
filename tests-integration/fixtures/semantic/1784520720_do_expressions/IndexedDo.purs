module IndexedDo where

foreign import data Render :: Type -> Type -> Type -> Type

data Unit = Unit

foreign import data Start :: Type

foreign import data Use1 :: Type -> Type
foreign import data Use2 :: Type -> Type

foreign import bind ::
  forall a b x y z.
  Render x y a ->
  (a -> Render y z b) ->
  Render x z b

foreign import discard ::
  forall a b x y z.
  Render x y a ->
  (a -> Render y z b) ->
  Render x z b

foreign import pure :: forall a x. a -> Render x x a

foreign import use1 :: forall hooks. Render hooks (Use1 hooks) Unit
foreign import use2 :: forall hooks. Render hooks (Use2 hooks) Unit
