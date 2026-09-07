module Main where

import Data.Bifunctor (class Bifunctor)

data ArrowSyntax a b = ArrowSyntax (a -> b)
derive instance Bifunctor ArrowSyntax

data ArrowConstructor a b = ArrowConstructor (Function a b)
derive instance Bifunctor ArrowConstructor
