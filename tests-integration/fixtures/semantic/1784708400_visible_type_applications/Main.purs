module Main where

identity :: forall @a. a -> a
identity value = value

const :: forall a @b. a -> b -> a
const value _ = value

testIdentity = identity @Int
testConst = const @String
