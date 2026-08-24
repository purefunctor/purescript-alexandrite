module Data.Function.Uncurried where

foreign import data Fn2 :: Type -> Type -> Type -> Type
foreign import data Fn3 :: Type -> Type -> Type -> Type -> Type

foreign import mkFn2 :: forall first second result. (first -> second -> result) -> Fn2 first second result
foreign import mkFn3 :: forall first second third result. (first -> second -> third -> result) -> Fn3 first second third result
foreign import runFn2 :: forall first second result. Fn2 first second result -> first -> second -> result
foreign import runFn3 :: forall first second third result. Fn3 first second third result -> first -> second -> third -> result
