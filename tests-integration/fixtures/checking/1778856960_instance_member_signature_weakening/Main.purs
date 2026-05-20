module Main where

import Data.Functor (class Functor)

class Transform :: ((Type -> Type) -> Type -> Type) -> Constraint
class Transform t where
  lift :: forall f a. Functor f => f a -> t f a

data Wrap :: (Type -> Type) -> Type -> Type
data Wrap f a = Wrap (f a)

instance Transform Wrap where
  lift :: forall f a. f a -> Wrap f a
  lift = Wrap

class Render a where
  render :: a -> String

class Renderer t where
  renderer :: forall a. Render a => a -> t

instance Renderer String where
  renderer :: forall a. Render a => a -> String
  renderer = render

class TransformPlain :: ((Type -> Type) -> Type -> Type) -> Constraint
class TransformPlain t where
  liftPlain :: forall f a. f a -> t f a

instance TransformPlain Wrap where
  liftPlain :: forall f a. Functor f => f a -> Wrap f a
  liftPlain = Wrap
