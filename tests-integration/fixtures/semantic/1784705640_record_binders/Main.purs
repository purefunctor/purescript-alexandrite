module Main where

project :: forall a b. { first :: a, second :: b } -> a
project { first, second: _ } = first
