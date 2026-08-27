module Data.Function.Uncurried where

foreign import data Fn2 :: Type -> Type -> Type -> Type

foreign import mkFn2 :: forall first second result. (first -> second -> result) -> Fn2 first second result
foreign import runFn2 :: forall first second result. Fn2 first second result -> first -> second -> result
