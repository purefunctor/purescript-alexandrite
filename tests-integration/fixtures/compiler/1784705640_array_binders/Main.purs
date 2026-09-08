module Main where

headOr :: forall a. a -> Array a -> a
headOr fallback [first, _] = first
headOr fallback _ = fallback
