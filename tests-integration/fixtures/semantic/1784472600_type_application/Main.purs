module Main where

foreign import identity :: forall @a. a -> a

test = identity @Int 0
