module Main where

ignore :: forall a b. a -> b -> b
ignore _ value = value
