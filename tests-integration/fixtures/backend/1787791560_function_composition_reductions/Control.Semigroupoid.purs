module Control.Semigroupoid where

class Semigroupoid (semigroupoid :: Type -> Type -> Type) where
  compose :: forall intermediate input output. semigroupoid intermediate output -> semigroupoid input intermediate -> semigroupoid input output

instance semigroupoidFn :: Semigroupoid Function where
  compose outer inner value = outer (inner value)

composeFlipped :: forall semigroupoid input intermediate output. Semigroupoid semigroupoid => semigroupoid input intermediate -> semigroupoid intermediate output -> semigroupoid input output
composeFlipped inner outer = compose outer inner
