module Main where

choose :: forall a b. a -> b -> a
choose left right = left

singleInfix :: Int
singleInfix = 1 `choose` "text"

multipleInfix :: Int
multipleInfix = 1 `choose` "first" `choose` true

identity :: forall a. a -> a
identity value = value

polymorphicHead = identity `choose` 1
