module Control.Semigroupoid where

class Semigroupoid (semigroupoid :: Type -> Type -> Type) where
  compose :: forall intermediate input output. semigroupoid intermediate output -> semigroupoid input intermediate -> semigroupoid input output

instance semigroupoidFn :: Semigroupoid Function where
  compose outer inner value = outer (inner value)
